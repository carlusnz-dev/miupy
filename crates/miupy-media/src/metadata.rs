//! Metadados de APIs externas: MusicBrainz, Cover Art Archive e LRCLIB.
//!
//! Regras de boa vizinhança: `User-Agent` identificado (`miupy/<versão> (<contato>)`), no máximo
//! 1 requisição por segundo ao MusicBrainz (implementado em [`crate::ratelimit`]) e timeouts em
//! tudo. Falha de rede devolve [`MediaError`], nunca `panic`.
//!
//! Biografia e foto não vêm do MusicBrainz: [`ArtistDetails::bio`] e
//! [`ArtistDetails::image_url`] ficam `None`. [`OnlineMetadata::artist_photo`] consulta Wikidata e
//! Wikimedia Commons quando há MBID e relação disponível.

use std::path::{Path, PathBuf};
use std::time::Duration;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;

use crate::error::{MediaError, Result};
use crate::ratelimit::RateLimiter;

const MAX_IMAGE_BYTES: usize = 15 * 1024 * 1024;
const MIN_MATCH_SCORE: u64 = 70;
const WIKIDATA_ENDPOINT: &str = "https://www.wikidata.org/w/api.php";
const COMMONS_ENDPOINT: &str = "https://commons.wikimedia.org/w/api.php";
const MAX_API_ATTEMPTS: usize = 3;
const MAX_RETRY_WAIT: Duration = Duration::from_secs(8);
const MAX_REQUEST_WINDOW: Duration = Duration::from_secs(15);

/// Gravação encontrada no MusicBrainz.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordingMatch {
    pub mbid: String,
    pub title: String,
    pub artist: String,
    pub artist_mbid: Option<String>,
    pub album: Option<String>,
    /// Release (para a capa no Cover Art Archive).
    pub release_mbid: Option<String>,
    /// Até 5 releases candidatos, do mais provável ao menos (oficiais e sem tipo secundário como
    /// "Compilation"/"Bootleg" primeiro). Nem todo release tem capa: tente em ordem.
    pub release_mbids: Vec<String>,
    pub duration_ms: Option<u64>,
    /// Pontuação do MusicBrainz, 0..=100.
    pub score: u32,
    /// Gêneros/tags por votos, em minúsculas, sem repetição (até 8).
    pub genres: Vec<String>,
    pub release_year: Option<u32>,
    pub isrc: Option<String>,
}

/// Dados de artista. `bio` e `image_url` ficam `None` com o MusicBrainz.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ArtistDetails {
    pub mbid: String,
    pub name: String,
    /// Código ISO 3166-1 ("FR"), se conhecido.
    pub country: Option<String>,
    pub genres: Vec<String>,
    pub bio: Option<String>,
    pub image_url: Option<String>,
}

// @req RF-27
/// Foto e crédito prontos para gravar em `covers/artistas/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtistPhoto {
    pub bytes: Vec<u8>,
    pub extension: String,
    /// `Autor — Licença`, sem marcação HTML.
    pub credit: String,
}

// @req RF-27
fn wikidata_id(mb: &Value) -> Option<String> {
    let url = mb
        .get("relations")?
        .as_array()?
        .iter()
        .find(|r| r.get("type").and_then(Value::as_str) == Some("wikidata"))?
        .pointer("/url/resource")?
        .as_str()?;
    let id = url.rsplit('/').next()?;
    (id.starts_with('Q') && id[1..].bytes().all(|b| b.is_ascii_digit()) && id.len() > 1)
        .then(|| id.to_string())
}

// @req RF-27
fn wikidata_image(wd: &Value, id: &str) -> Option<String> {
    wd.pointer(&format!(
        "/entities/{id}/claims/P18/0/mainsnak/datavalue/value"
    ))?
    .as_str()
    .map(String::from)
}

// @req RF-27
fn commons_image(commons: &Value) -> Option<(String, String)> {
    let page = commons
        .pointer("/query/pages")?
        .as_object()?
        .values()
        .next()?;
    let info = page.pointer("/imageinfo/0")?;
    let url = info
        .get("thumburl")
        .or_else(|| info.get("url"))?
        .as_str()?
        .to_string();
    let meta = info.get("extmetadata")?;
    let artist = meta
        .pointer("/Artist/value")
        .and_then(Value::as_str)
        .map(plain_text)
        .unwrap_or_default();
    let license = meta
        .pointer("/LicenseShortName/value")
        .and_then(Value::as_str)
        .map(plain_text)
        .unwrap_or_default();
    let credit = match (artist.is_empty(), license.is_empty()) {
        (false, false) => format!("{artist} — {license}"),
        (false, true) => artist,
        (true, false) => license,
        (true, true) => "Wikimedia Commons".into(),
    };
    Some((url, credit))
}

// @req RF-27
fn plain_text(html: &str) -> String {
    let mut out = String::new();
    let mut inside = false;
    for c in html.chars() {
        match c {
            '<' => inside = true,
            '>' => inside = false,
            _ if !inside => out.push(c),
            _ => {}
        }
    }
    out.replace("&amp;", "&")
        .replace("&nbsp;", " ")
        .replace("&#39;", "'")
        .replace("&quot;", "\"")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Letra de uma música.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LyricsData {
    pub plain: Option<String>,
    /// Formato LRC (`[mm:ss.xx] texto`).
    pub synced: Option<String>,
    pub instrumental: bool,
}

/// Fonte de metadados. Trocável/combinável (ver docs do módulo).
#[async_trait]
pub trait MetadataProvider: Send + Sync {
    async fn artist_photo(&self, _mbid: Option<&str>) -> Result<Option<ArtistPhoto>> {
        Ok(None)
    }
    /// Melhor gravação para título (+ artista, + duração para desempate). `None` se nada
    /// atingir a pontuação mínima.
    async fn find_recording(
        &self,
        title: &str,
        artist: Option<&str>,
        duration_ms: Option<u64>,
    ) -> Result<Option<RecordingMatch>>;

    /// Busca artista pelo nome e traz seus detalhes.
    async fn find_artist(&self, name: &str) -> Result<Option<ArtistDetails>>;

    /// Detalhes de um artista pelo mbid.
    async fn artist_details(&self, mbid: &str) -> Result<ArtistDetails>;

    /// Baixa a capa frontal do release para `dest_dir/<stem>.<ext>`.
    async fn download_cover(
        &self,
        release_mbid: &str,
        dest_dir: &Path,
        file_stem: &str,
    ) -> Result<PathBuf>;

    /// Letra por título/artista (+ álbum e duração, que melhoram o acerto).
    async fn lyrics(
        &self,
        title: &str,
        artist: &str,
        album: Option<&str>,
        duration_ms: Option<u64>,
    ) -> Result<Option<LyricsData>>;

    /// Baixa uma imagem qualquer (ex.: miniatura do YouTube) para `dest_dir/<stem>.<ext>`.
    async fn download_image(&self, url: &str, dest_dir: &Path, file_stem: &str) -> Result<PathBuf>;
}

/// Configuração de [`OnlineMetadata`].
#[derive(Debug, Clone)]
pub struct MetadataConfig {
    /// Contato incluído no User-Agent (e-mail ou URL). Obrigatório pelo MusicBrainz.
    pub contact: String,
    pub musicbrainz_url: String,
    pub coverart_url: String,
    pub lrclib_url: String,
    pub timeout: Duration,
    /// Intervalo mínimo entre requisições ao MusicBrainz.
    pub musicbrainz_interval: Duration,
    /// Intervalo mínimo entre requisições ao LRCLIB.
    pub lrclib_interval: Duration,
}

impl MetadataConfig {
    pub fn new(contact: impl Into<String>) -> Self {
        Self {
            contact: contact.into(),
            musicbrainz_url: "https://musicbrainz.org/ws/2".into(),
            coverart_url: "https://coverartarchive.org".into(),
            lrclib_url: "https://lrclib.net/api".into(),
            timeout: Duration::from_secs(20),
            musicbrainz_interval: Duration::from_secs(1),
            lrclib_interval: Duration::from_secs(2),
        }
    }
}

/// Implementação com `reqwest` (rustls).
pub struct OnlineMetadata {
    http: reqwest::Client,
    cfg: MetadataConfig,
    mb_limiter: RateLimiter,
    lrclib_limiter: RateLimiter,
    user_agent: String,
}

impl OnlineMetadata {
    // @req RF-27
    /// Busca foto pelo MBID e devolve bytes, extensão e crédito. Ausência de vínculo ou
    /// imagem devolve `None`; chamadas ao MusicBrainz compartilham o limitador existente.
    pub async fn artist_photo(&self, mbid: Option<&str>) -> Result<Option<ArtistPhoto>> {
        let Some(mbid) = mbid.filter(|s| !s.is_empty()) else {
            return Ok(None);
        };
        validate_mbid(mbid)?;
        let mb = self
            .mb_get(
                &format!("artist/{mbid}"),
                &[("inc", "url-rels"), ("fmt", "json")],
            )
            .await?;
        let Some(item) = wikidata_id(&mb) else {
            return Ok(None);
        };
        let mut wd_url = reqwest::Url::parse(WIKIDATA_ENDPOINT)
            .map_err(|e| MediaError::Internal(format!("URL do Wikidata inválida: {e}")))?;
        wd_url.query_pairs_mut().extend_pairs([
            ("action", "wbgetentities"),
            ("ids", item.as_str()),
            ("props", "claims"),
            ("format", "json"),
        ]);
        let wd = self.http.get(wd_url).send().await?;
        let wd: Value = Self::check(wd, "Wikidata").await?.json().await?;
        let Some(file) = wikidata_image(&wd, &item) else {
            return Ok(None);
        };
        let mut commons_url = reqwest::Url::parse(COMMONS_ENDPOINT)
            .map_err(|e| MediaError::Internal(format!("URL do Commons inválida: {e}")))?;
        commons_url.query_pairs_mut().extend_pairs([
            ("action", "query"),
            ("titles", format!("File:{file}").as_str()),
            ("prop", "imageinfo"),
            ("iiprop", "url|extmetadata"),
            ("iiurlwidth", "500"),
            ("format", "json"),
        ]);
        let commons = self.http.get(commons_url).send().await?;
        let commons: Value = Self::check(commons, "Commons").await?.json().await?;
        let Some((url, credit)) = commons_image(&commons) else {
            return Ok(None);
        };
        let parsed = reqwest::Url::parse(&url).map_err(|e| MediaError::Internal(e.to_string()))?;
        if !matches!(parsed.scheme(), "http" | "https") {
            return Err(MediaError::Internal("URL da foto inválida".into()));
        }
        let response = Self::check(self.http.get(parsed).send().await?, "foto do artista").await?;
        if response
            .content_length()
            .is_some_and(|n| n > MAX_IMAGE_BYTES as u64)
        {
            return Err(MediaError::InvalidInput("foto grande demais".into()));
        }
        let bytes = response.bytes().await?;
        if bytes.len() > MAX_IMAGE_BYTES {
            return Err(MediaError::InvalidInput("foto grande demais".into()));
        }
        let extension = image_extension(&bytes, None)
            .ok_or_else(|| MediaError::Internal("foto em formato desconhecido".into()))?;
        Ok(Some(ArtistPhoto {
            bytes: bytes.to_vec(),
            extension: extension.into(),
            credit,
        }))
    }
    // @req RF-12
    /// Falha com `InvalidInput` se o contato for vazio (o User-Agent identificado é obrigatório).
    pub fn new(cfg: MetadataConfig) -> Result<Self> {
        let contact = cfg.contact.trim();
        if contact.is_empty() || contact.chars().any(char::is_control) {
            return Err(MediaError::InvalidInput(
                "contato do User-Agent vazio ou inválido (ex.: e-mail)".into(),
            ));
        }
        let user_agent = format!("miupy/{} ({})", env!("CARGO_PKG_VERSION"), contact);
        let http = reqwest::Client::builder()
            .user_agent(&user_agent)
            .timeout(cfg.timeout)
            .connect_timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| MediaError::Internal(format!("cliente HTTP: {e}")))?;
        Ok(Self {
            http,
            mb_limiter: RateLimiter::new(cfg.musicbrainz_interval),
            lrclib_limiter: RateLimiter::new(cfg.lrclib_interval),
            cfg,
            user_agent,
        })
    }

    /// User-Agent enviado em todas as requisições.
    pub fn user_agent(&self) -> &str {
        &self.user_agent
    }

    fn url(base: &str, path: &str, params: &[(&str, &str)]) -> Result<reqwest::Url> {
        let mut url = reqwest::Url::parse(&format!("{}/{}", base.trim_end_matches('/'), path))
            .map_err(|e| MediaError::Internal(format!("URL inválida: {e}")))?;
        url.query_pairs_mut().extend_pairs(params.iter().copied());
        Ok(url)
    }

    async fn check(resp: reqwest::Response, what: &str) -> Result<reqwest::Response> {
        let status = resp.status();
        if status.is_success() {
            Ok(resp)
        } else if status == reqwest::StatusCode::NOT_FOUND {
            Err(MediaError::NotFound(what.to_string()))
        } else if status == reqwest::StatusCode::SERVICE_UNAVAILABLE
            || status == reqwest::StatusCode::TOO_MANY_REQUESTS
        {
            Err(MediaError::Network(format!(
                "{what}: serviço pediu para reduzir o ritmo (HTTP {status})"
            )))
        } else {
            Err(MediaError::Network(format!("{what}: HTTP {status}")))
        }
    }

    async fn mb_get(&self, path: &str, params: &[(&str, &str)]) -> Result<Value> {
        let url = Self::url(&self.cfg.musicbrainz_url, path, params)?;
        let resp = self
            .request_limited(url, &self.mb_limiter, "MusicBrainz")
            .await?;
        Self::check(resp, "MusicBrainz")
            .await?
            .json()
            .await
            .map_err(Into::into)
    }

    async fn lrclib_get<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        params: &[(&str, &str)],
    ) -> Result<T> {
        let url = Self::url(&self.cfg.lrclib_url, path, params)?;
        let resp = self
            .request_limited(url, &self.lrclib_limiter, "LRCLIB")
            .await?;
        Self::check(resp, "LRCLIB")
            .await?
            .json()
            .await
            .map_err(Into::into)
    }

    // @req RF-12
    /// Cada tentativa reserva uma vaga nova; uma espera excessiva do servidor encerra a busca.
    async fn request_limited(
        &self,
        url: reqwest::Url,
        limiter: &RateLimiter,
        what: &str,
    ) -> Result<reqwest::Response> {
        tokio::time::timeout(
            MAX_REQUEST_WINDOW,
            self.request_limited_inner(url, limiter, what),
        )
        .await
        .map_err(|_| {
            MediaError::Network(format!("{what}: tempo esgotado após várias tentativas"))
        })?
    }

    async fn request_limited_inner(
        &self,
        url: reqwest::Url,
        limiter: &RateLimiter,
        what: &str,
    ) -> Result<reqwest::Response> {
        for attempt in 0..MAX_API_ATTEMPTS {
            limiter.acquire().await;
            let response = self.http.get(url.clone()).send().await;
            let retry_after = match response {
                Ok(resp) if !matches!(resp.status().as_u16(), 429 | 503) => return Ok(resp),
                Ok(resp) => {
                    if attempt + 1 == MAX_API_ATTEMPTS {
                        return Ok(resp);
                    }
                    let wait = retry_after(&resp);
                    if wait.is_some_and(|d| d > MAX_RETRY_WAIT) {
                        return Self::check(resp, what).await;
                    }
                    wait
                }
                Err(e) => {
                    if attempt + 1 == MAX_API_ATTEMPTS {
                        return Err(e.into());
                    }
                    None
                }
            };
            let backoff = Duration::from_secs(1 << attempt);
            tokio::time::sleep(backoff.max(retry_after.unwrap_or_default())).await;
        }
        unreachable!("o laço sempre devolve na última tentativa")
    }
}

// @req RF-12
fn retry_after(resp: &reqwest::Response) -> Option<Duration> {
    let raw = resp
        .headers()
        .get(reqwest::header::RETRY_AFTER)?
        .to_str()
        .ok()?;
    retry_after_value(raw)
}

fn retry_after_value(raw: &str) -> Option<Duration> {
    if let Ok(seconds) = raw.parse::<u64>() {
        return Some(Duration::from_secs(seconds));
    }
    let date = chrono::DateTime::parse_from_rfc2822(raw).ok()?;
    let seconds = u64::try_from(date.timestamp()).ok()?;
    Some(
        (std::time::UNIX_EPOCH + Duration::from_secs(seconds))
            .duration_since(std::time::SystemTime::now())
            .unwrap_or_default(),
    )
}

#[async_trait]
impl MetadataProvider for OnlineMetadata {
    async fn artist_photo(&self, mbid: Option<&str>) -> Result<Option<ArtistPhoto>> {
        OnlineMetadata::artist_photo(self, mbid).await
    }
    // @req RF-12 RF-20
    async fn find_recording(
        &self,
        title: &str,
        artist: Option<&str>,
        duration_ms: Option<u64>,
    ) -> Result<Option<RecordingMatch>> {
        let query = recording_query(title, artist)?;
        let json = self
            .mb_get(
                "recording",
                &[("query", &query), ("fmt", "json"), ("limit", "10")],
            )
            .await?;
        let Some(mut recording) = parse_recordings(&json, duration_ms) else {
            return Ok(None);
        };
        let details = self
            .mb_get(
                &format!("recording/{}", recording.mbid),
                &[("inc", "genres tags isrcs"), ("fmt", "json")],
            )
            .await?;
        recording.genres = ranked_genres(&details);
        recording.release_year = release_year(&details).or(recording.release_year);
        recording.isrc = details
            .get("isrcs")
            .and_then(Value::as_array)
            .and_then(|items| items.iter().filter_map(Value::as_str).next())
            .map(String::from);
        Ok(Some(recording))
    }

    // @req RF-12
    async fn find_artist(&self, name: &str) -> Result<Option<ArtistDetails>> {
        let name = name.trim();
        if name.is_empty() {
            return Err(MediaError::InvalidInput("nome de artista vazio".into()));
        }
        let query = format!("artist:\"{}\"", escape_phrase(name));
        let json = self
            .mb_get(
                "artist",
                &[("query", &query), ("fmt", "json"), ("limit", "3")],
            )
            .await?;
        match parse_artist_search(&json) {
            Some(mbid) => self.artist_details(&mbid).await.map(Some),
            None => Ok(None),
        }
    }

    // @req RF-12
    async fn artist_details(&self, mbid: &str) -> Result<ArtistDetails> {
        validate_mbid(mbid)?;
        let json = self
            .mb_get(
                &format!("artist/{mbid}"),
                &[("inc", "genres tags"), ("fmt", "json")],
            )
            .await?;
        parse_artist(&json)
    }

    // @req RF-12
    async fn download_cover(
        &self,
        release_mbid: &str,
        dest_dir: &Path,
        file_stem: &str,
    ) -> Result<PathBuf> {
        validate_mbid(release_mbid)?;
        let url = format!(
            "{}/release/{release_mbid}/front-500",
            self.cfg.coverart_url.trim_end_matches('/')
        );
        self.download_image(&url, dest_dir, file_stem).await
    }

    // @req RF-12
    async fn lyrics(
        &self,
        title: &str,
        artist: &str,
        album: Option<&str>,
        duration_ms: Option<u64>,
    ) -> Result<Option<LyricsData>> {
        let (title, artist) = (title.trim(), artist.trim());
        if title.is_empty() || artist.is_empty() {
            return Err(MediaError::InvalidInput(
                "título e artista são obrigatórios".into(),
            ));
        }
        // 1) /get exato (exige álbum e duração).
        if let (Some(album), Some(ms)) = (album.filter(|a| !a.trim().is_empty()), duration_ms) {
            let secs = (ms / 1000).to_string();
            match self
                .lrclib_get::<LrclibItem>(
                    "get",
                    &[
                        ("track_name", title),
                        ("artist_name", artist),
                        ("album_name", album),
                        ("duration", &secs),
                    ],
                )
                .await
            {
                Ok(item) => {
                    if let Some(l) = item.into_lyrics() {
                        return Ok(Some(l));
                    }
                }
                Err(MediaError::NotFound(_)) => {}
                Err(e) => return Err(e),
            }
        }
        // 2) busca aproximada.
        let items: Vec<LrclibItem> = self
            .lrclib_get("search", &[("track_name", title), ("artist_name", artist)])
            .await?;
        Ok(pick_lyrics(items, duration_ms))
    }

    // @req RF-09 RF-12
    async fn download_image(&self, url: &str, dest_dir: &Path, file_stem: &str) -> Result<PathBuf> {
        let parsed = reqwest::Url::parse(url)
            .map_err(|_| MediaError::InvalidInput("URL de imagem inválida".into()))?;
        if !matches!(parsed.scheme(), "http" | "https") {
            return Err(MediaError::InvalidInput(
                "a URL da imagem precisa ser http(s)".into(),
            ));
        }
        crate::ytdlp::validate_file_stem(file_stem)?;
        let resp = self.http.get(parsed).send().await?;
        let resp = Self::check(resp, "imagem").await?;
        let content_type = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        if resp
            .content_length()
            .is_some_and(|n| n as usize > MAX_IMAGE_BYTES)
        {
            return Err(MediaError::InvalidInput("imagem grande demais".into()));
        }
        let bytes = resp.bytes().await?;
        if bytes.len() > MAX_IMAGE_BYTES {
            return Err(MediaError::InvalidInput("imagem grande demais".into()));
        }
        let ext = image_extension(&bytes, content_type.as_deref()).ok_or_else(|| {
            MediaError::Internal("a resposta não é uma imagem reconhecida".into())
        })?;
        tokio::fs::create_dir_all(dest_dir).await?;
        let final_path = dest_dir.join(format!("{file_stem}.{ext}"));
        let tmp = dest_dir.join(format!("{file_stem}.{ext}.part"));
        tokio::fs::write(&tmp, &bytes).await?;
        tokio::fs::rename(&tmp, &final_path).await?;
        Ok(final_path)
    }
}

// ---------------------------------------------------------------------------------------------
// Funções puras (testadas com fixtures)
// ---------------------------------------------------------------------------------------------

/// Escapa `"` e `\` para uso dentro de uma frase entre aspas na sintaxe Lucene.
pub(crate) fn escape_phrase(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

pub(crate) fn recording_query(title: &str, artist: Option<&str>) -> Result<String> {
    let title = title.trim();
    if title.is_empty() {
        return Err(MediaError::InvalidInput("título vazio".into()));
    }
    let mut q = format!("recording:\"{}\"", escape_phrase(title));
    if let Some(a) = artist.map(str::trim).filter(|a| !a.is_empty()) {
        q.push_str(&format!(" AND artist:\"{}\"", escape_phrase(a)));
    }
    Ok(q)
}

fn validate_mbid(s: &str) -> Result<()> {
    let ok = s.len() == 36 && s.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-');
    if ok {
        Ok(())
    } else {
        Err(MediaError::InvalidInput(format!("mbid inválido: {s:?}")))
    }
}

fn str_of<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key).and_then(Value::as_str)
}

/// Escolhe a melhor gravação: pontuação mínima e, havendo duração, a mais próxima (≤ 10 s).
pub(crate) fn parse_recordings(json: &Value, duration_ms: Option<u64>) -> Option<RecordingMatch> {
    let candidates: Vec<RecordingMatch> = json
        .get("recordings")?
        .as_array()?
        .iter()
        .filter_map(|r| {
            let score = r.get("score").and_then(Value::as_u64).unwrap_or(0);
            if score < MIN_MATCH_SCORE {
                return None;
            }
            let credits = r.get("artist-credit").and_then(Value::as_array);
            let artist = credits
                .map(|c| {
                    c.iter()
                        .map(|x| {
                            let name = str_of(x, "name").unwrap_or("");
                            let join = str_of(x, "joinphrase").unwrap_or("");
                            format!("{name}{join}")
                        })
                        .collect::<String>()
                })
                .unwrap_or_default();
            let artist_mbid = credits
                .and_then(|c| c.first())
                .and_then(|x| x.get("artist"))
                .and_then(|a| str_of(a, "id"))
                .map(String::from);
            let mut releases: Vec<&Value> = r
                .get("releases")
                .and_then(Value::as_array)
                .map(|a| a.iter().collect())
                .unwrap_or_default();
            // `sort_by_key` é estável: dentro do mesmo grupo vale a ordem do MusicBrainz.
            releases.sort_by_key(|x| {
                let official = str_of(x, "status") == Some("Official");
                let plain = x
                    .get("release-group")
                    .and_then(|g| g.get("secondary-types"))
                    .and_then(Value::as_array)
                    .is_none_or(|t| t.is_empty());
                (!official, !plain)
            });
            let release = releases.first().copied();
            let release_mbids = releases
                .iter()
                .filter_map(|x| str_of(x, "id"))
                .take(5)
                .map(String::from)
                .collect();
            Some(RecordingMatch {
                mbid: str_of(r, "id")?.to_string(),
                title: str_of(r, "title")?.to_string(),
                artist,
                artist_mbid,
                album: release.and_then(|x| str_of(x, "title")).map(String::from),
                release_mbid: release.and_then(|x| str_of(x, "id")).map(String::from),
                release_mbids,
                duration_ms: r.get("length").and_then(Value::as_u64),
                score: score as u32,
                genres: ranked_genres(r),
                release_year: release_year(r),
                isrc: r
                    .get("isrcs")
                    .and_then(Value::as_array)
                    .and_then(|items| items.iter().filter_map(Value::as_str).next())
                    .map(String::from),
            })
        })
        .collect();
    if let Some(target) = duration_ms {
        let close = candidates
            .iter()
            .filter_map(|c| c.duration_ms.map(|d| (d.abs_diff(target), c)))
            .filter(|(diff, _)| *diff <= 10_000)
            .min_by_key(|(diff, _)| *diff)
            .map(|(_, c)| c.clone());
        return close.or_else(|| {
            candidates
                .iter()
                .all(|candidate| candidate.duration_ms.is_none())
                .then(|| candidates.first().cloned())
                .flatten()
        });
    }
    candidates.into_iter().next()
}

pub(crate) fn parse_artist_search(json: &Value) -> Option<String> {
    json.get("artists")?
        .as_array()?
        .iter()
        .find(|a| a.get("score").and_then(Value::as_u64).unwrap_or(0) >= 90)
        .and_then(|a| str_of(a, "id"))
        .map(String::from)
}

pub(crate) fn parse_artist(json: &Value) -> Result<ArtistDetails> {
    let mbid = str_of(json, "id")
        .ok_or_else(|| MediaError::Internal("artista sem id na resposta".into()))?
        .to_string();
    let name = str_of(json, "name").unwrap_or_default().to_string();
    let genres = ranked_genres(json);
    Ok(ArtistDetails {
        mbid,
        name,
        country: str_of(json, "country").map(String::from),
        genres,
        bio: None,
        image_url: None,
    })
}

// @req RF-20
fn ranked_genres(json: &Value) -> Vec<String> {
    let mut items: Vec<(u64, String)> = ["genres", "tags"]
        .into_iter()
        .flat_map(|key| {
            json.get(key)
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
        })
        .filter_map(|item| {
            Some((
                item.get("count").and_then(Value::as_u64).unwrap_or(0),
                str_of(item, "name")?.trim().to_lowercase(),
            ))
        })
        .filter(|(_, name)| !name.is_empty())
        .collect();
    items.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    let mut seen = std::collections::HashSet::new();
    items
        .into_iter()
        .map(|(_, name)| name)
        .filter(|name| seen.insert(name.clone()))
        .take(8)
        .collect()
}

fn release_year(json: &Value) -> Option<u32> {
    str_of(json, "first-release-date")
        .or_else(|| str_of(json, "date"))
        .and_then(|date| date.get(..4))
        .and_then(|year| year.parse::<u32>().ok())
        .filter(|year| (1000..=3000).contains(year))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LrclibItem {
    #[serde(default)]
    duration: Option<f64>,
    #[serde(default)]
    instrumental: bool,
    #[serde(default)]
    plain_lyrics: Option<String>,
    #[serde(default)]
    synced_lyrics: Option<String>,
}

impl LrclibItem {
    fn into_lyrics(self) -> Option<LyricsData> {
        let plain = self.plain_lyrics.filter(|s| !s.trim().is_empty());
        let synced = self.synced_lyrics.filter(|s| !s.trim().is_empty());
        if plain.is_none() && synced.is_none() && !self.instrumental {
            return None;
        }
        Some(LyricsData {
            plain,
            synced,
            instrumental: self.instrumental,
        })
    }
}

/// Entre os resultados da busca: com letra, o de duração mais próxima (≤ 3 s se a duração for
/// conhecida), preferindo ter versão sincronizada.
pub(crate) fn pick_lyrics(items: Vec<LrclibItem>, duration_ms: Option<u64>) -> Option<LyricsData> {
    let target = duration_ms.map(|d| d as f64 / 1000.0);
    let mut best: Option<(f64, LrclibItem)> = None;
    for it in items {
        let has = it
            .plain_lyrics
            .as_deref()
            .is_some_and(|s| !s.trim().is_empty())
            || it
                .synced_lyrics
                .as_deref()
                .is_some_and(|s| !s.trim().is_empty());
        if !has {
            continue;
        }
        let diff = match (target, it.duration) {
            (Some(t), Some(d)) => (t - d).abs(),
            _ => 0.0,
        };
        if target.is_some() && it.duration.is_some() && diff > 3.0 {
            continue;
        }
        let synced_bonus = if it.synced_lyrics.is_some() {
            -0.5
        } else {
            0.0
        };
        let score = diff + synced_bonus;
        if best.as_ref().is_none_or(|(s, _)| score < *s) {
            best = Some((score, it));
        }
    }
    best.and_then(|(_, it)| it.into_lyrics())
}

/// Extensão pelo conteúdo (magic bytes), com o Content-Type como reserva.
pub(crate) fn image_extension(bytes: &[u8], content_type: Option<&str>) -> Option<&'static str> {
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("jpg")
    } else if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        Some("png")
    } else if bytes.len() > 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("webp")
    } else if bytes.starts_with(b"GIF8") {
        Some("gif")
    } else {
        match content_type.map(|c| c.split(';').next().unwrap_or("").trim()) {
            Some("image/jpeg") => Some("jpg"),
            Some("image/png") => Some("png"),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    fn fixture(s: &str) -> Value {
        serde_json::from_str(s).unwrap()
    }
    const MB_REC: &str = include_str!("../tests/fixtures/musicbrainz_recording.json");
    const MB_ART: &str = include_str!("../tests/fixtures/musicbrainz_artist.json");
    const MB_ART_SEARCH: &str = include_str!("../tests/fixtures/musicbrainz_artist_search.json");
    const LRC_GET: &str = include_str!("../tests/fixtures/lrclib_get.json");
    const LRC_SEARCH: &str = include_str!("../tests/fixtures/lrclib_search.json");

    // @test RF-12
    #[test]
    fn consulta_lucene_escapa_aspas() {
        assert_eq!(
            recording_query("Say \"Hi\" \\ x", Some("A & B")).unwrap(),
            "recording:\"Say \\\"Hi\\\" \\\\ x\" AND artist:\"A & B\""
        );
        assert_eq!(recording_query("x", None).unwrap(), "recording:\"x\"");
        assert!(recording_query("  ", None).is_err());
    }

    // @test RF-12
    #[test]
    fn parse_gravacao() {
        let m = parse_recordings(&fixture(MB_REC), None).unwrap();
        assert_eq!(m.mbid, "b1a9c0e9-d987-4042-ae91-78d6a3267d69");
        assert_eq!(m.title, "Get Lucky");
        assert_eq!(m.artist, "Daft Punk feat. Pharrell Williams");
        assert_eq!(
            m.artist_mbid.as_deref(),
            Some("056e4f3e-d505-4dad-8ec1-d04f521cbb56")
        );
        assert_eq!(m.album.as_deref(), Some("Random Access Memories"));
        assert_eq!(
            m.release_mbid.as_deref(),
            Some("a0b5b3c2-1111-4c2a-9b5e-000000000001")
        );
        assert_eq!(m.release_mbids.len(), 3);
        assert_eq!(m.release_mbids[0], "a0b5b3c2-1111-4c2a-9b5e-000000000001");
        assert_eq!(m.release_mbids[2], "a0b5b3c2-1111-4c2a-9b5e-0000000000ff");
        assert_eq!(m.duration_ms, Some(369_000));
        assert_eq!(m.score, 100);
        // duração distante não atribui o lançamento de outra gravação à faixa.
        assert!(parse_recordings(&fixture(MB_REC), Some(10_000)).is_none());
        assert!(parse_recordings(&fixture(r#"{"recordings":[]}"#), None).is_none());
        assert!(parse_recordings(&fixture(r#"{"x":1}"#), None).is_none());
        let baixa = fixture(r#"{"recordings":[{"id":"a","title":"t","score":10}]}"#);
        assert!(parse_recordings(&baixa, None).is_none());
    }

    // @test RF-20
    #[test]
    fn generos_ordenados_sem_repeticao_e_ano() {
        let value = fixture(
            r#"{"first-release-date":"2007-04-02","genres":[{"name":"Rock","count":3},{"name":"JAZZ","count":8}],"tags":[{"name":"rock","count":2},{"name":"Funk","count":5}]}"#,
        );
        assert_eq!(ranked_genres(&value), ["jazz", "funk", "rock"]);
        assert_eq!(release_year(&value), Some(2007));
    }

    // @test RF-12
    #[test]
    fn parse_artista() {
        let a = parse_artist(&fixture(MB_ART)).unwrap();
        assert_eq!(a.mbid, "056e4f3e-d505-4dad-8ec1-d04f521cbb56");
        assert_eq!(a.name, "Daft Punk");
        assert_eq!(a.country.as_deref(), Some("FR"));
        assert_eq!(a.genres, vec!["house", "electronic", "french", "funk"]);
        assert_eq!(a.bio, None);
        assert_eq!(a.image_url, None);
        // sem genres, usa tags
        let b = parse_artist(&fixture(
            r#"{"id":"x","name":"N","tags":[{"name":"rock","count":2},{"name":"pop","count":9}]}"#,
        ))
        .unwrap();
        assert_eq!(b.genres, vec!["pop", "rock"]);
        assert_eq!(b.country, None);
        assert!(parse_artist(&fixture("{}")).is_err());
        assert_eq!(
            parse_artist_search(&fixture(MB_ART_SEARCH)).as_deref(),
            Some("056e4f3e-d505-4dad-8ec1-d04f521cbb56")
        );
        assert!(parse_artist_search(&fixture(r#"{"artists":[{"id":"a","score":40}]}"#)).is_none());
    }

    // @test RF-12
    #[test]
    fn parse_letra_get_e_busca() {
        let item: LrclibItem = serde_json::from_str(LRC_GET).unwrap();
        let l = item.into_lyrics().unwrap();
        assert!(l.plain.unwrap().starts_with("I feel your breath"));
        assert!(l.synced.unwrap().starts_with("[00:17.12]"));
        assert!(!l.instrumental);

        let items: Vec<LrclibItem> = serde_json::from_str(LRC_SEARCH).unwrap();
        // duração 181s: pega o item 2 (181.4 s), ignora o de 300 s e o instrumental
        let l = pick_lyrics(items, Some(181_000)).unwrap();
        assert_eq!(l.plain.as_deref(), Some("plain certa"));
        assert_eq!(l.synced.as_deref(), Some("[00:01.00] sync certa"));
        // sem duração conhecida, ainda escolhe algo com letra (prefere sincronizada)
        let items: Vec<LrclibItem> = serde_json::from_str(LRC_SEARCH).unwrap();
        assert_eq!(
            pick_lyrics(items, None).unwrap().plain.as_deref(),
            Some("plain certa")
        );
        // nenhuma dentro de 3 s
        let items: Vec<LrclibItem> = serde_json::from_str(LRC_SEARCH).unwrap();
        assert!(pick_lyrics(items, Some(60_000)).is_none());
        // instrumental sem letra
        let i: LrclibItem =
            serde_json::from_str(r#"{"instrumental":true,"plainLyrics":null}"#).unwrap();
        assert!(i.into_lyrics().unwrap().instrumental);
        let vazio: LrclibItem = serde_json::from_str(r#"{"plainLyrics":"  "}"#).unwrap();
        assert!(vazio.into_lyrics().is_none());
    }

    // @test RF-12
    #[test]
    fn extensao_de_imagem() {
        assert_eq!(
            image_extension(&[0xFF, 0xD8, 0xFF, 0xE0], None),
            Some("jpg")
        );
        assert_eq!(
            image_extension(b"\x89PNG\r\n\x1a\n", Some("text/html")),
            Some("png")
        );
        assert_eq!(image_extension(b"RIFF\0\0\0\0WEBPVP8 ", None), Some("webp"));
        assert_eq!(image_extension(b"<html>", Some("text/html")), None);
        assert_eq!(
            image_extension(b"xx", Some("image/jpeg; charset=x")),
            Some("jpg")
        );
    }

    // @test RF-27
    #[test]
    fn fixtures_wikidata_e_commons_extraem_imagem_e_credito_sem_html() {
        let mb: Value =
            serde_json::from_str(include_str!("../tests/fixtures/artist_photo_mb.json")).unwrap();
        let wd: Value =
            serde_json::from_str(include_str!("../tests/fixtures/artist_photo_wikidata.json"))
                .unwrap();
        let commons: Value =
            serde_json::from_str(include_str!("../tests/fixtures/artist_photo_commons.json"))
                .unwrap();
        let id = wikidata_id(&mb).unwrap();
        assert!(id.starts_with('Q'));
        let file = wikidata_image(&wd, &id).unwrap();
        assert_eq!(file, "Daft Punk in 2013.jpg");
        let (url, credit) = commons_image(&commons).unwrap();
        assert!(url.contains("500px"));
        assert_eq!(credit, "Sony Music Entertainment — CC BY 4.0");
        assert!(!credit.contains('<'));
    }

    // @test RF-12
    #[test]
    fn contato_obrigatorio_e_user_agent() {
        assert!(OnlineMetadata::new(MetadataConfig::new("  ")).is_err());
        let m = OnlineMetadata::new(MetadataConfig::new("eu@example.com")).unwrap();
        assert_eq!(
            m.user_agent(),
            format!("miupy/{} (eu@example.com)", env!("CARGO_PKG_VERSION"))
        );
    }

    /// Servidor HTTP mínimo: responde `responses` em ordem (uma por conexão) e devolve as
    /// requisições recebidas.
    async fn mock_server(
        responses: Vec<(u16, &'static str, Vec<u8>)>,
    ) -> (String, tokio::task::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let h = tokio::spawn(async move {
            let mut seen = Vec::new();
            for (status, ctype, body) in responses {
                let (mut s, _) = listener.accept().await.unwrap();
                let mut buf = vec![0u8; 8192];
                let n = s.read(&mut buf).await.unwrap();
                seen.push(String::from_utf8_lossy(&buf[..n]).into_owned());
                let head = format!(
                    "HTTP/1.1 {status} X\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                s.write_all(head.as_bytes()).await.unwrap();
                s.write_all(&body).await.unwrap();
                let _ = s.shutdown().await;
            }
            seen
        });
        (format!("http://{addr}"), h)
    }

    /// Servidor com cabeçalhos opcionais e horário virtual de cada chamada.
    async fn timed_server(
        responses: Vec<(u16, &'static str, &'static str)>,
    ) -> (String, tokio::task::JoinHandle<Vec<tokio::time::Instant>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let h = tokio::spawn(async move {
            let mut times = Vec::new();
            for (status, headers, body) in responses {
                let (mut s, _) = listener.accept().await.unwrap();
                times.push(tokio::time::Instant::now());
                let mut buf = [0u8; 8192];
                let n = s.read(&mut buf).await.unwrap();
                assert!(n > 0);
                let head = format!(
                    "HTTP/1.1 {status} X\r\n{headers}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                s.write_all(head.as_bytes()).await.unwrap();
                s.write_all(body.as_bytes()).await.unwrap();
            }
            times
        });
        (format!("http://{addr}"), h)
    }

    // @test RF-12
    #[tokio::test(start_paused = true)]
    async fn musicbrainz_repete_503_respeitando_retry_after_e_limite() {
        let (base, h) =
            timed_server(vec![(503, "Retry-After: 2\r\n", "{}"), (200, "", MB_ART)]).await;
        let p = provider(&base);
        let result = p
            .artist_details("056e4f3e-d505-4dad-8ec1-d04f521cbb56")
            .await;
        assert_eq!(result.unwrap().name, "Daft Punk");
        let times = h.await.unwrap();
        assert!(times[1].duration_since(times[0]) >= Duration::from_secs(2));
    }

    // @test RF-12
    #[tokio::test(start_paused = true)]
    async fn lrclib_repete_429_e_respeita_intervalo_proprio() {
        let (base, h) = timed_server(vec![
            (429, "Retry-After: 2\r\n", "{}"),
            (200, "", LRC_SEARCH),
            (200, "", LRC_SEARCH),
        ])
        .await;
        let p = provider(&base);
        assert!(
            p.lyrics("Song", "A", None, Some(181_000))
                .await
                .unwrap()
                .is_some()
        );
        assert!(
            p.lyrics("Song", "A", None, Some(181_000))
                .await
                .unwrap()
                .is_some()
        );
        let times = h.await.unwrap();
        assert!(times[1].duration_since(times[0]) >= Duration::from_secs(2));
        assert!(times[2].duration_since(times[1]) >= Duration::from_secs(2));
    }

    // @test RF-12
    #[tokio::test(start_paused = true)]
    async fn lrclib_para_apos_tres_tentativas() {
        let (base, h) = timed_server(vec![(503, "", "{}"), (503, "", "{}"), (503, "", "{}")]).await;
        let err = provider(&base)
            .lyrics("Song", "A", None, None)
            .await
            .unwrap_err();
        assert_eq!(err.code(), "network");
        assert_eq!(h.await.unwrap().len(), 3);
    }

    // @test RF-12
    #[tokio::test(start_paused = true)]
    async fn retry_after_longo_encerra_sem_desrespeitar_o_servidor() {
        let (base, h) = timed_server(vec![(503, "Retry-After: 60\r\n", "{}")]).await;
        let err = provider(&base)
            .lyrics("Song", "A", None, None)
            .await
            .unwrap_err();
        assert_eq!(err.code(), "network");
        assert_eq!(h.await.unwrap().len(), 1);
    }

    // @test RF-12
    #[test]
    fn retry_after_aceita_segundos_e_data_http() {
        assert_eq!(retry_after_value("3"), Some(Duration::from_secs(3)));
        assert!(
            retry_after_value("Wed, 14 Oct 2037 12:00:00 GMT").unwrap() > Duration::from_secs(8)
        );
        assert_eq!(
            retry_after_value("Wed, 14 Oct 2020 12:00:00 GMT"),
            Some(Duration::ZERO)
        );
        assert_eq!(retry_after_value("inválido"), None);
    }

    fn provider(base: &str) -> OnlineMetadata {
        let mut cfg = MetadataConfig::new("teste@example.com");
        cfg.musicbrainz_url = format!("{base}/ws/2");
        cfg.coverart_url = base.to_string();
        cfg.lrclib_url = format!("{base}/api");
        cfg.timeout = Duration::from_secs(3);
        cfg.musicbrainz_interval = Duration::from_millis(1);
        OnlineMetadata::new(cfg).unwrap()
    }

    // @test RF-12 RF-20
    #[tokio::test]
    async fn http_envia_user_agent_e_le_resposta() {
        let detail = include_str!("../tests/fixtures/musicbrainz_recording_detail.json");
        let (base, h) = mock_server(vec![
            (200, "application/json", MB_REC.as_bytes().to_vec()),
            (200, "application/json", detail.as_bytes().to_vec()),
        ])
        .await;
        let m = provider(&base)
            .find_recording("Get Lucky", Some("Daft Punk"), Some(369_000))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(m.title, "Get Lucky");
        assert_eq!(m.release_year, Some(2015));
        let requests = h.await.unwrap();
        assert!(requests[1].contains("inc=genres+tags+isrcs"));
        let req = requests[0].to_lowercase();
        assert!(req.contains("user-agent: miupy/"), "{req}");
        assert!(req.contains("teste@example.com"));
        assert!(
            req.starts_with("get /ws/2/recording?query=recording%3a%22get+lucky"),
            "{req}"
        );
    }

    // @test RF-12
    #[tokio::test]
    async fn http_404_e_503_viram_erros_tipados() {
        let (base, _h) = mock_server(vec![
            (404, "text/plain", b"nf".to_vec()),
            (503, "text/plain", b"slow down".to_vec()),
        ])
        .await;
        let p = provider(&base);
        let e = p
            .download_cover(
                "b1a9c0e9-d987-4042-ae91-78d6a3267d69",
                Path::new("/nonexistent-dir"),
                "c",
            )
            .await
            .unwrap_err();
        assert_eq!(e.code(), "not_found");
        let e = p
            .artist_details("056e4f3e-d505-4dad-8ec1-d04f521cbb56")
            .await
            .unwrap_err();
        assert_eq!(e.code(), "network");
    }

    // @test RF-12
    #[tokio::test]
    async fn http_falha_de_conexao_nao_da_panic() {
        // porta fechada
        let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = l.local_addr().unwrap();
        drop(l);
        let p = provider(&format!("http://{addr}"));
        let e = p.find_recording("x", None, None).await.unwrap_err();
        assert_eq!(e.code(), "network");
        let e = p.lyrics("a", "b", None, None).await.unwrap_err();
        assert_eq!(e.code(), "network");
    }

    // @test RF-12
    #[tokio::test]
    async fn http_resposta_invalida_vira_internal() {
        let (base, _h) =
            mock_server(vec![(200, "application/json", b"{nao e json".to_vec())]).await;
        let e = provider(&base)
            .find_recording("x", None, None)
            .await
            .unwrap_err();
        assert_eq!(e.code(), "internal");
    }

    // @test RF-09 RF-12
    #[tokio::test]
    async fn download_de_imagem_grava_arquivo_com_extensao_correta() {
        let png = b"\x89PNG\r\n\x1a\n0000".to_vec();
        let (base, _h) = mock_server(vec![
            (200, "application/octet-stream", png.clone()),
            (200, "text/html", b"<html>".to_vec()),
        ])
        .await;
        let dir = tempfile::tempdir().unwrap();
        let p = provider(&base);
        let path = p
            .download_image(&format!("{base}/thumb"), dir.path(), "capa1")
            .await
            .unwrap();
        assert_eq!(path, dir.path().join("capa1.png"));
        assert_eq!(std::fs::read(&path).unwrap(), png);
        assert!(!dir.path().join("capa1.png.part").exists());
        let e = p
            .download_image(&format!("{base}/x"), dir.path(), "capa2")
            .await
            .unwrap_err();
        assert_eq!(e.code(), "internal");
        assert_eq!(
            p.download_image("file:///etc/passwd", dir.path(), "c")
                .await
                .unwrap_err()
                .code(),
            "invalid_input"
        );
        assert_eq!(
            p.download_image(&format!("{base}/x"), dir.path(), "../c")
                .await
                .unwrap_err()
                .code(),
            "invalid_input"
        );
    }

    // @test RF-12
    #[tokio::test]
    async fn letra_usa_get_e_cai_para_busca_no_404() {
        let (base, h) = mock_server(vec![
            (404, "application/json", b"{}".to_vec()),
            (200, "application/json", LRC_SEARCH.as_bytes().to_vec()),
        ])
        .await;
        let l = provider(&base)
            .lyrics("Song", "A", Some("Y"), Some(181_000))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(l.plain.as_deref(), Some("plain certa"));
        let reqs = h.await.unwrap();
        assert!(reqs[0].starts_with("GET /api/get?"), "{}", reqs[0]);
        assert!(reqs[0].contains("duration=181"));
        assert!(reqs[1].starts_with("GET /api/search?"));
    }

    // @test RF-12
    #[tokio::test(start_paused = true)]
    async fn musicbrainz_respeita_um_por_segundo() {
        // Tempo pausado: as 3 chamadas ao MusicBrainz precisam de >= 2 s de relógio virtual.
        let mut cfg = MetadataConfig::new("t@example.com");
        cfg.musicbrainz_url = "http://127.0.0.1:1/ws/2".into(); // recusa na hora
        let p = OnlineMetadata::new(cfg).unwrap();
        let t0 = tokio::time::Instant::now();
        for _ in 0..3 {
            let _ = p.find_recording("x", None, None).await;
        }
        assert!(t0.elapsed() >= Duration::from_secs(2), "{:?}", t0.elapsed());
    }
}
