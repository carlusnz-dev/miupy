//! Implementação de [`MediaResolver`] que executa o binário `yt-dlp` como processo.
//!
//! O caminho real é verificado por testes de rede ignorados por padrão.
//!
//! Segurança: a linha de comando é sempre uma lista de argumentos separados (nunca passa por
//! shell). A consulta do usuário vai dentro do valor `ytsearchN:<consulta>`, que nunca começa
//! com `-`, então não pode ser lida como opção. URLs/ids são validados e remontados por nós.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::Command;

use crate::error::{MediaError, Result};
use crate::metadata::MetadataProvider;
use crate::resolver::{
    DownloadRequest, DownloadedAudio, MediaResolver, PlaylistInfo, SearchResult, StreamAudio,
    VideoMetadata,
};
use crate::tagger;
use crate::title::clean_title;

const SEARCH_TIMEOUT: Duration = Duration::from_secs(60);
const PLAYLIST_TIMEOUT: Duration = Duration::from_secs(180);
const VIDEO_TIMEOUT: Duration = Duration::from_secs(90);
const MAX_LIMIT: usize = 50;
const MAX_QUERY_LEN: usize = 300;
const PROGRESS_PREFIX: &str = "MIUPYPROG";
const FILE_PREFIX: &str = "MIUPYFILE";
/// M4A/AAC 128 kbps (itag 140); `bestaudio[ext=m4a]` como alternativa. Sem fallback para
/// webm/opus: o motor de áudio (symphonia) não decodifica Opus.
const AUDIO_FORMAT: &str = "140/bestaudio[ext=m4a]";

/// Resolver baseado no executável `yt-dlp`.
#[derive(Debug, Clone)]
pub struct YtDlpResolver {
    binary: PathBuf,
}

impl Default for YtDlpResolver {
    fn default() -> Self {
        Self::new()
    }
}

/// `ETXTBSY` ("Text file busy"); o mesmo número no Linux, macOS e BSDs.
const ETXTBSY: i32 = 26;

// @req RF-10 RF-26
/// Repete o `spawn` quando o executável acabou de ser gravado e ainda está "ocupado".
///
/// Logo depois de gravar o binário (instalação/atualização do yt-dlp, ou o script dos testes),
/// outra thread que faz `fork` pode herdar o descritor de escrita até o `exec` do filho dela; nesse
/// intervalo o `exec` do binário falha com `ETXTBSY`. A janela é curta: algumas tentativas resolvem.
async fn retry_text_busy<T>(mut spawn: impl FnMut() -> std::io::Result<T>) -> std::io::Result<T> {
    let mut attempt = 0;
    loop {
        match spawn() {
            Err(e) if e.raw_os_error() == Some(ETXTBSY) && attempt < 5 => {
                attempt += 1;
                tokio::time::sleep(Duration::from_millis(20 * attempt)).await;
            }
            other => return other,
        }
    }
}

impl YtDlpResolver {
    /// Usa `yt-dlp` do `PATH`.
    pub fn new() -> Self {
        Self::with_binary("yt-dlp")
    }

    /// Usa um executável em caminho específico (ex.: baixado pelo próprio app).
    pub fn with_binary(path: impl Into<PathBuf>) -> Self {
        Self {
            binary: path.into(),
        }
    }

    pub fn binary(&self) -> &Path {
        &self.binary
    }

    // @req RF-09 RF-20
    /// Reconsulta o vídeo de uma faixa existente e devolve os campos corrigidos.
    /// O provedor é opcional; se presente, confirma título, álbum e ano no MusicBrainz.
    pub async fn reprocess_track(
        &self,
        source_id: &str,
        provider: Option<&dyn MetadataProvider>,
    ) -> Result<CorrectedTrack> {
        let video = self.video_json(source_id, false).await?;
        let mut corrected = corrected_track(&video);
        if let Some(provider) = provider
            && let Some(found) = provider
                .find_recording(
                    &corrected.title,
                    Some(&corrected.artist),
                    duration_ms(&video),
                )
                .await?
        {
            corrected.title = found.title;
            corrected.artist = found.artist;
            corrected.album = found.album.or(corrected.album);
            corrected.release_year = found.release_year.or(corrected.release_year);
        }
        Ok(corrected)
    }

    // @req RF-09
    /// Roda `yt-dlp --version`. Devolve a versão, ou `Unavailable` explicando como instalar.
    pub async fn check_available(&self) -> Result<String> {
        let out = self
            .run_capture(vec!["--version".into()], Duration::from_secs(15))
            .await?;
        Ok(out.trim().to_string())
    }

    fn unavailable(&self) -> MediaError {
        MediaError::Unavailable(format!(
            "yt-dlp não encontrado em `{}`. Instale com `pipx install yt-dlp`, `brew install \
             yt-dlp`, `winget install yt-dlp` ou baixe de \
             https://github.com/yt-dlp/yt-dlp/releases e informe o caminho do executável.",
            self.binary.display()
        ))
    }

    fn map_spawn_error(&self, e: std::io::Error) -> MediaError {
        match e.kind() {
            std::io::ErrorKind::NotFound => self.unavailable(),
            std::io::ErrorKind::PermissionDenied => MediaError::Unavailable(format!(
                "sem permissão para executar `{}` (chmod +x?)",
                self.binary.display()
            )),
            _ => MediaError::Io(format!("falha ao iniciar yt-dlp: {e}")),
        }
    }

    async fn run_capture(&self, args: Vec<String>, timeout: Duration) -> Result<String> {
        let mut cmd = Command::new(&self.binary);
        cmd.args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        let fut = async {
            retry_text_busy(|| cmd.spawn())
                .await?
                .wait_with_output()
                .await
        };
        let out = match tokio::time::timeout(timeout, fut).await {
            Err(_) => return Err(MediaError::Network("yt-dlp excedeu o tempo limite".into())),
            Ok(Err(e)) => return Err(self.map_spawn_error(e)),
            Ok(Ok(o)) => o,
        };
        if !out.status.success() {
            return Err(classify_stderr(&String::from_utf8_lossy(&out.stderr)));
        }
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    }
}

#[async_trait]
impl MediaResolver for YtDlpResolver {
    // @req RF-09
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let wanted = limit.clamp(1, MAX_LIMIT);
        let args = search_args(query, (wanted * 3).min(150))?;
        let out = self.run_capture(args, SEARCH_TIMEOUT).await?;
        Ok(parse_search_output(&out)
            .into_iter()
            .filter(is_song_result)
            .take(wanted)
            .collect())
    }

    // @req RF-10
    async fn download_audio(&self, request: DownloadRequest) -> Result<DownloadedAudio> {
        validate_video_id(&request.video_id)?;
        validate_file_stem(&request.file_stem)?;
        tokio::fs::create_dir_all(&request.dest_dir).await?;
        let args = download_args(&request.video_id, &request.dest_dir, &request.file_stem);

        let mut cmd = Command::new(&self.binary);
        cmd.args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        let mut child = retry_text_busy(|| cmd.spawn())
            .await
            .map_err(|e| self.map_spawn_error(e))?;

        let stdout = child.stdout.take().expect("stdout piped");
        let mut stderr = child.stderr.take().expect("stderr piped");
        let stderr_task = tokio::spawn(async move {
            let mut buf = Vec::new();
            let _ = (&mut stderr).take(64 * 1024).read_to_end(&mut buf).await;
            String::from_utf8_lossy(&buf).into_owned()
        });

        let mut lines = BufReader::new(stdout).lines();
        let mut printed_path: Option<PathBuf> = None;
        loop {
            tokio::select! {
                _ = request.cancel.cancelled() => {
                    let _ = child.kill().await;
                    let _ = child.wait().await;
                    stderr_task.abort();
                    cleanup_partials(&request.dest_dir, &request.file_stem).await;
                    return Err(MediaError::Cancelled);
                }
                line = lines.next_line() => match line {
                    Ok(Some(line)) => {
                        if let Some(p) = parse_progress_line(&line) {
                            if let Some(cb) = &request.progress {
                                cb(p);
                            }
                        } else if let Some(path) = parse_file_line(&line) {
                            printed_path = Some(path);
                        }
                    }
                    Ok(None) => break,
                    Err(e) => {
                        let _ = child.kill().await;
                        cleanup_partials(&request.dest_dir, &request.file_stem).await;
                        return Err(MediaError::Io(format!("leitura da saída do yt-dlp: {e}")));
                    }
                }
            }
        }

        // O stdout fechou; espera o término, ainda respeitando o cancelamento.
        let status = tokio::select! {
            _ = request.cancel.cancelled() => {
                let _ = child.kill().await;
                let _ = child.wait().await;
                cleanup_partials(&request.dest_dir, &request.file_stem).await;
                return Err(MediaError::Cancelled);
            }
            s = child.wait() => s?,
        };
        let stderr_text = stderr_task.await.unwrap_or_default();
        if !status.success() {
            cleanup_partials(&request.dest_dir, &request.file_stem).await;
            return Err(classify_stderr(&stderr_text));
        }

        let path = printed_path
            .unwrap_or_else(|| request.dest_dir.join(format!("{}.m4a", request.file_stem)));
        if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
            return Err(MediaError::Internal(format!(
                "yt-dlp terminou sem erro, mas o arquivo não existe: {}",
                path.display()
            )));
        }
        let info_path = path.clone();
        let info = tokio::task::spawn_blocking(move || tagger::read_tags(&info_path))
            .await
            .map_err(|e| MediaError::Internal(e.to_string()))??;
        if let Some(cb) = &request.progress {
            cb(1.0);
        }
        Ok(DownloadedAudio {
            path,
            codec: info.codec,
            bitrate_kbps: info.bitrate_kbps,
            duration_ms: info.duration_ms,
        })
    }

    // @req RF-11
    async fn resolve_playlist(&self, url_or_id: &str) -> Result<PlaylistInfo> {
        let id = playlist_id(url_or_id)?;
        let args = playlist_args(&id);
        let out = self.run_capture(args, PLAYLIST_TIMEOUT).await?;
        parse_playlist_output(&out)
    }

    // @req RF-18
    async fn resolve_stream(&self, video_id: &str) -> Result<StreamAudio> {
        let json = self.video_json(video_id, true).await?;
        parse_stream(&json)
    }

    // @req RF-19
    async fn related_tracks(&self, video_id: &str, limit: usize) -> Result<Vec<SearchResult>> {
        validate_video_id(video_id)?;
        let mut args = base_args();
        args.extend([
            "--flat-playlist".into(),
            "--dump-single-json".into(),
            "--playlist-end".into(),
            (limit.clamp(1, MAX_LIMIT) * 3 + 1).to_string(),
            "--".into(),
            // O mix só responde pela URL do vídeo com `list=RD<id>`; a forma
            // `playlist?list=RD<id>` devolve "This playlist type is unviewable".
            format!("https://www.youtube.com/watch?v={video_id}&list=RD{video_id}"),
        ]);
        let out = match self.run_capture(args, PLAYLIST_TIMEOUT).await {
            Ok(out) => out,
            Err(MediaError::NotFound(_)) | Err(MediaError::Unavailable(_)) => return Ok(Vec::new()),
            Err(MediaError::Internal(message)) if message.contains("unviewable") => {
                return Ok(Vec::new());
            }
            Err(e) => return Err(e),
        };
        Ok(filter_related(
            parse_playlist_output(&out)?.entries,
            video_id,
            limit,
        ))
    }

    // @req RF-20
    async fn video_metadata(&self, video_id: &str) -> Result<VideoMetadata> {
        let json = self.video_json(video_id, false).await?;
        Ok(parse_video_metadata(&json))
    }
}

impl YtDlpResolver {
    async fn video_json(&self, video_id: &str, audio_only: bool) -> Result<Value> {
        validate_video_id(video_id)?;
        let mut args = base_args();
        args.extend([
            "--no-playlist".into(),
            "--dump-single-json".into(),
            "--skip-download".into(),
        ]);
        if audio_only {
            args.extend(["-f".into(), AUDIO_FORMAT.into()]);
        }
        args.extend([
            "--".into(),
            format!("https://www.youtube.com/watch?v={video_id}"),
        ]);
        let out = self.run_capture(args, VIDEO_TIMEOUT).await?;
        serde_json::from_str(&out)
            .map_err(|e| MediaError::Internal(format!("JSON do vídeo ilegível: {e}")))
    }
}

// ---------------------------------------------------------------------------------------------
// Validação e construção de argumentos (puras, testadas)
// ---------------------------------------------------------------------------------------------

fn base_args() -> Vec<String> {
    ["--ignore-config", "--no-warnings", "--no-color"]
        .into_iter()
        .map(String::from)
        .collect()
}

pub(crate) fn search_args(query: &str, limit: usize) -> Result<Vec<String>> {
    let q = query.trim();
    if q.is_empty() {
        return Err(MediaError::InvalidInput("consulta vazia".into()));
    }
    if q.chars().count() > MAX_QUERY_LEN || q.chars().any(char::is_control) {
        return Err(MediaError::InvalidInput(
            "consulta longa demais ou com caracteres de controle".into(),
        ));
    }
    let limit = limit.clamp(1, MAX_LIMIT * 3);
    let mut args = base_args();
    args.extend(["--flat-playlist".into(), "--dump-json".into()]);
    // Consulta começando com `-` é segura: o argumento inteiro começa com "ytsearch".
    args.push(format!("ytsearch{limit}:{q}"));
    Ok(args)
}

pub(crate) fn playlist_args(playlist_id: &str) -> Vec<String> {
    let mut args = base_args();
    args.extend(["--flat-playlist".into(), "--dump-single-json".into()]);
    args.push("--".into());
    args.push(format!(
        "https://www.youtube.com/playlist?list={playlist_id}"
    ));
    args
}

pub(crate) fn download_args(video_id: &str, dest_dir: &Path, file_stem: &str) -> Vec<String> {
    let mut args = base_args();
    let template = dest_dir.join(format!("{file_stem}.%(ext)s"));
    args.extend([
        "--no-playlist".into(),
        "-f".into(),
        AUDIO_FORMAT.into(),
        "--no-mtime".into(),
        "--no-simulate".into(),
        "--quiet".into(),
        "--progress".into(),
        "--newline".into(),
        "--progress-template".into(),
        format!(
            "download:{PROGRESS_PREFIX} %(progress.downloaded_bytes)s \
             %(progress.total_bytes)s %(progress.total_bytes_estimate)s"
        ),
        "--print".into(),
        format!("after_move:{FILE_PREFIX}\t%(filepath)s"),
        "-o".into(),
        template.to_string_lossy().into_owned(),
        "--".into(),
        format!("https://www.youtube.com/watch?v={video_id}"),
    ]);
    args
}

/// Id de vídeo do YouTube: 11 caracteres `[A-Za-z0-9_-]`.
pub(crate) fn validate_video_id(id: &str) -> Result<()> {
    let ok = id.len() == 11
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-');
    if ok {
        Ok(())
    } else {
        Err(MediaError::InvalidInput(format!(
            "id de vídeo inválido: {id:?}"
        )))
    }
}

pub(crate) fn validate_file_stem(stem: &str) -> Result<()> {
    let bad = stem.is_empty()
        || stem.starts_with('.')
        || stem.len() > 200
        || stem
            .chars()
            .any(|c| matches!(c, '/' | '\\' | '\0' | '%') || c.is_control());
    if bad {
        Err(MediaError::InvalidInput(format!(
            "nome de arquivo inválido: {stem:?}"
        )))
    } else {
        Ok(())
    }
}

/// Extrai o id de playlist de uma URL do YouTube (`?list=`) ou valida um id solto.
pub(crate) fn playlist_id(input: &str) -> Result<String> {
    let input = input.trim();
    let valid = |s: &str| {
        (10..=80).contains(&s.len())
            && s.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    };
    if input.contains("://") {
        let url = reqwest::Url::parse(input)
            .map_err(|_| MediaError::InvalidInput("URL de playlist inválida".into()))?;
        let host = url.host_str().unwrap_or("");
        let is_yt = host == "youtu.be" || host == "youtube.com" || host.ends_with(".youtube.com");
        if !is_yt || !matches!(url.scheme(), "http" | "https") {
            return Err(MediaError::InvalidInput("a URL não é do YouTube".into()));
        }
        let list = url
            .query_pairs()
            .find(|(k, _)| k == "list")
            .map(|(_, v)| v.into_owned())
            .ok_or_else(|| MediaError::InvalidInput("a URL não tem o parâmetro list=".into()))?;
        if valid(&list) {
            return Ok(list);
        }
        return Err(MediaError::InvalidInput("id de playlist inválido".into()));
    }
    if valid(input) {
        Ok(input.to_string())
    } else {
        Err(MediaError::InvalidInput("id de playlist inválido".into()))
    }
}

// ---------------------------------------------------------------------------------------------
// Parsing de saída (puro, testado com fixtures)
// ---------------------------------------------------------------------------------------------

fn parse_entry(v: &Value) -> Option<SearchResult> {
    let id = v.get("id")?.as_str()?;
    if validate_video_id(id).is_err() {
        return None;
    }
    let title = v.get("title")?.as_str()?.trim().to_string();
    let channel = ["channel", "uploader"]
        .iter()
        .find_map(|k| v.get(*k).and_then(Value::as_str))
        .unwrap_or("")
        .to_string();
    let duration_ms = v
        .get("duration")
        .and_then(Value::as_f64)
        .filter(|d| *d >= 0.0)
        .map(|d| (d * 1000.0).round() as u64);
    // Prefere a maior miniatura que não seja webp (o tagger só embute jpeg/png).
    let thumb = v
        .get("thumbnails")
        .and_then(Value::as_array)
        .and_then(|a| {
            a.iter()
                .rev()
                .filter_map(|t| t.get("url").and_then(Value::as_str))
                .find(|u| !u.contains(".webp") && !u.contains("/vi_webp/"))
        })
        .map(String::from)
        .unwrap_or_else(|| format!("https://i.ytimg.com/vi/{id}/hqdefault.jpg"));
    Some(SearchResult {
        video_id: id.to_string(),
        title,
        channel,
        duration_ms,
        thumbnail_url: Some(thumb),
    })
}

// @req RF-18
fn parse_stream(v: &Value) -> Result<StreamAudio> {
    let codec = v.get("acodec").and_then(Value::as_str).unwrap_or("");
    let container = v.get("ext").and_then(Value::as_str).unwrap_or("");
    if container != "m4a" || !codec.starts_with("mp4a.") {
        return Err(MediaError::Unavailable(
            "formato M4A/AAC indisponível para streaming".into(),
        ));
    }
    let url = v
        .get("url")
        .and_then(Value::as_str)
        .ok_or_else(|| MediaError::Unavailable("vídeo sem URL direta de áudio".into()))?;
    let parsed = reqwest::Url::parse(url)
        .map_err(|_| MediaError::Internal("URL de áudio inválida".into()))?;
    if parsed.scheme() != "https" {
        return Err(MediaError::Unavailable("URL de áudio sem HTTPS".into()));
    }
    let expires_at = parsed
        .query_pairs()
        .find(|(key, _)| key == "expire")
        .and_then(|(_, value)| value.parse().ok());
    let http_headers = v
        .get("http_headers")
        .and_then(Value::as_object)
        .map(|headers| {
            headers
                .iter()
                .filter_map(|(key, value)| {
                    value.as_str().map(|value| (key.clone(), value.to_string()))
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(StreamAudio {
        url: url.to_string(),
        http_headers,
        codec: "aac".into(),
        container: container.into(),
        bitrate_kbps: v
            .get("abr")
            .and_then(Value::as_f64)
            .filter(|n| *n >= 0.0)
            .map(|n| n.round() as u32),
        duration_ms: v
            .get("duration")
            .and_then(Value::as_f64)
            .filter(|n| *n >= 0.0)
            .map(|n| (n * 1000.0).round() as u64),
        expires_at,
    })
}

// @req RF-20
fn parse_video_metadata(v: &Value) -> VideoMetadata {
    let field = |key| {
        v.get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
    };
    let mut genres = Vec::new();
    let mut seen = HashSet::new();
    for key in ["categories", "tags"] {
        if let Some(items) = v.get(key).and_then(Value::as_array) {
            for item in items {
                if let Some(name) = item.as_str().map(str::trim).filter(|s| !s.is_empty()) {
                    let name = name.to_lowercase();
                    if seen.insert(name.clone()) {
                        genres.push(name);
                    }
                }
            }
        }
    }
    genres.truncate(8);
    VideoMetadata {
        channel: field("channel").or_else(|| field("uploader")),
        raw_title: field("title"),
        artist: field("artist").or_else(|| field("creator")),
        track: field("track"),
        album: field("album"),
        release_year: v
            .get("release_year")
            .and_then(Value::as_u64)
            .and_then(|year| u32::try_from(year).ok())
            .filter(|year| (1000..=3000).contains(year))
            .or_else(|| {
                ["release_date"]
                    .iter()
                    .filter_map(|key| v.get(*key).and_then(Value::as_str))
                    .find_map(|date| date.get(..4)?.parse::<u32>().ok())
                    .filter(|year| (1000..=3000).contains(year))
            }),
        genres,
    }
}

/// Campos prontos para atualizar uma faixa já salva no banco.
// @req RF-09 RF-20
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorrectedTrack {
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub release_year: Option<u32>,
}

// @req RF-09 RF-20
/// Extrai campos musicais de um JSON de vídeo do yt-dlp. Campos estruturados têm prioridade.
pub fn corrected_track(v: &Value) -> CorrectedTrack {
    let field = |key| {
        v.get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
    };
    let raw = field("title").unwrap_or("");
    let channel = field("channel").or_else(|| field("uploader")).unwrap_or("");
    let parsed = clean_title(raw, channel);
    let parsed_artist = parsed.artist_credit();
    let mut album = field("album").map(String::from);
    let parts: Vec<_> = raw.split(" - ").map(str::trim).collect();
    if album.is_none() && parts.len() >= 3 && parts.iter().all(|s| !s.is_empty()) {
        album = Some(parts[1..parts.len() - 1].join(" - "));
    } else if album.is_none()
        && parts.len() == 2
        && parts[1].split_whitespace().count() >= 6
        && parsed.title.eq_ignore_ascii_case(parts[0])
    {
        album = Some(parts[1].to_string());
    }
    let meta = parse_video_metadata(v);
    CorrectedTrack {
        title: field("track").unwrap_or(&parsed.title).to_string(),
        artist: field("artist")
            .map(String::from)
            .or_else(|| {
                v.get("artists")
                    .and_then(Value::as_array)
                    .map(|artists| {
                        artists
                            .iter()
                            .filter_map(Value::as_str)
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .filter(|artists| !artists.is_empty())
            })
            .or_else(|| field("creator").map(String::from))
            .or(parsed_artist)
            .unwrap_or_else(|| channel.to_string()),
        album,
        release_year: meta.release_year,
    }
}

fn duration_ms(v: &Value) -> Option<u64> {
    v.get("duration")
        .and_then(Value::as_f64)
        .filter(|d| *d >= 0.0)
        .map(|d| (d * 1000.0).round() as u64)
}

// @req RF-09
/// Decide se um resultado é uma faixa avulsa, usando duração e termos de coletânea.
pub fn is_song_result(entry: &SearchResult) -> bool {
    if entry.duration_ms.is_some_and(|ms| ms > 600_000) {
        return false;
    }
    let title = entry.title.to_lowercase();
    let normalized = title.replace(['_', '|', '[', ']', '(', ')', '–', '—', '-', ':', '/'], " ");
    let words: Vec<&str> = normalized.split_whitespace().collect();
    let contains = |phrase: &[&str]| words.windows(phrase.len()).any(|w| w == phrase);
    if [
        &["full", "album"][..],
        &["album", "completo"],
        &["álbum", "completo"],
        &["greatest", "hits"],
        &["1", "hour"],
        &["1", "hora"],
        &["one", "hour"],
        &["coletânea"],
        &["compilação"],
        &["compilation"],
        &["playlist"],
    ]
    .iter()
    .any(|phrase| contains(phrase))
    {
        return false;
    }
    // "Mix" também nomeia versões de uma faixa ("Radio Mix", "Extended Mix"); só indica
    // coletânea quando o vídeo é longo (ou sem duração conhecida) e o título tem mais termos.
    let long_or_unknown = entry.duration_ms.is_none_or(|ms| ms > 420_000);
    !(words.contains(&"mix") && words.len() > 2 && long_or_unknown)
}

fn filter_related(entries: Vec<SearchResult>, seed: &str, limit: usize) -> Vec<SearchResult> {
    let mut seen = HashSet::new();
    entries
        .into_iter()
        .filter(|entry| {
            entry.video_id != seed && seen.insert(entry.video_id.clone()) && is_song_result(entry)
        })
        .take(limit.clamp(1, MAX_LIMIT))
        .collect()
}

/// Uma linha JSON por resultado (`--dump-json`). Linhas inválidas são ignoradas.
pub(crate) fn parse_search_output(stdout: &str) -> Vec<SearchResult> {
    stdout
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .filter_map(|v| parse_entry(&v))
        .collect()
}

pub(crate) fn parse_playlist_output(stdout: &str) -> Result<PlaylistInfo> {
    let v: Value = serde_json::from_str(stdout)
        .map_err(|e| MediaError::Internal(format!("JSON da playlist ilegível: {e}")))?;
    let id = v
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let title = v.get("title").and_then(Value::as_str).map(String::from);
    let entries = v
        .get("entries")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(parse_entry).collect())
        .unwrap_or_default();
    Ok(PlaylistInfo { id, title, entries })
}

/// `MIUPYPROG <baixado> <total> <total_estimado>` -> fração 0..1.
pub(crate) fn parse_progress_line(line: &str) -> Option<f32> {
    let rest = line.trim().strip_prefix(PROGRESS_PREFIX)?;
    let mut it = rest.split_whitespace();
    let num = |s: Option<&str>| s.and_then(|s| s.parse::<f64>().ok()).filter(|n| *n >= 0.0);
    let downloaded = num(it.next())?;
    let total = num(it.next()).filter(|t| *t > 0.0);
    let estimate = num(it.next()).filter(|t| *t > 0.0);
    let total = total.or(estimate)?;
    Some((downloaded / total).clamp(0.0, 1.0) as f32)
}

/// `MIUPYFILE\t<caminho>` (impresso após mover para o destino final).
pub(crate) fn parse_file_line(line: &str) -> Option<PathBuf> {
    let path = line.strip_prefix(FILE_PREFIX)?.strip_prefix('\t')?.trim();
    (!path.is_empty() && path != "NA").then(|| PathBuf::from(path))
}

/// Classifica a mensagem de erro do yt-dlp.
pub(crate) fn classify_stderr(stderr: &str) -> MediaError {
    let last = stderr
        .lines()
        .rev()
        .find(|l| l.contains("ERROR"))
        .or_else(|| stderr.lines().rev().find(|l| !l.trim().is_empty()))
        .unwrap_or("yt-dlp falhou sem mensagem")
        .trim()
        .to_string();
    let l = last.to_lowercase();
    if l.contains("requested format is not available") || l.contains("no video formats found") {
        MediaError::Unavailable(format!("formato M4A indisponível para este vídeo: {last}"))
    } else if l.contains("sign in to confirm your age")
        || l.contains("age-restricted")
        || l.contains("not available in your country")
        || l.contains("not available in your region")
        || l.contains("geo-restricted")
        || l.contains("this video is unavailable in your country")
    {
        MediaError::Unavailable(last)
    } else if l.contains("video unavailable")
        || l.contains("private video")
        || l.contains("this video is not available")
        || l.contains("has been removed")
        || l.contains("does not exist")
        || l.contains("http error 404")
        || l.contains("this video is unavailable")
    {
        MediaError::NotFound(last)
    } else if l.contains("unable to download")
        || l.contains("urlopen error")
        || l.contains("name resolution")
        || l.contains("timed out")
        || l.contains("connection")
        || l.contains("http error 5")
        || l.contains("http error 429")
        || l.contains("network is unreachable")
        || l.contains("unable to extract") && l.contains("network")
    {
        MediaError::Network(last)
    } else if l.contains("playlist type is unviewable") || l.contains("playlist does not exist") {
        MediaError::NotFound(last)
    } else {
        MediaError::Internal(last)
    }
}

/// `true` para arquivos parciais do download de `stem` (`.part`, `.ytdl`, `.temp`, `.f140.m4a`).
pub(crate) fn is_partial_name(stem: &str, name: &str) -> bool {
    let Some(rest) = name.strip_prefix(stem).and_then(|r| r.strip_prefix('.')) else {
        return false;
    };
    if rest.ends_with(".part") || rest.ends_with(".ytdl") || rest.ends_with(".temp") {
        return true;
    }
    // stem.f140.m4a (formatos antes do merge)
    rest.strip_prefix('f')
        .and_then(|r| r.split('.').next())
        .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

async fn cleanup_partials(dir: &Path, stem: &str) {
    let Ok(mut rd) = tokio::fs::read_dir(dir).await else {
        return;
    };
    while let Ok(Some(entry)) = rd.next_entry().await {
        if let Some(name) = entry.file_name().to_str()
            && is_partial_name(stem, name)
        {
            let _ = tokio::fs::remove_file(entry.path()).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEARCH: &str = include_str!("../tests/fixtures/ytdlp_search.jsonl");
    const PLAYLIST: &str = include_str!("../tests/fixtures/ytdlp_playlist.json");
    const VIDEO: &str = include_str!("../tests/fixtures/ytdlp_video.json");
    const PROGRESS: &str = include_str!("../tests/fixtures/ytdlp_progress.txt");
    const ERRORS: &str = include_str!("../tests/fixtures/ytdlp_errors.json");

    // @test RF-18 RF-20
    #[test]
    fn video_capturado_resolve_stream_e_metadados() {
        let video: Value = serde_json::from_str(VIDEO).unwrap();
        let stream = parse_stream(&video).unwrap();
        assert_eq!(stream.codec, "aac");
        assert_eq!(stream.container, "m4a");
        assert_eq!(stream.duration_ms, Some(19_000));
        assert!(stream.bitrate_kbps.unwrap() >= 100);
        assert_eq!(stream.expires_at, Some(1791910000));
        assert!(stream.http_headers.contains_key("User-Agent"));
        let meta = parse_video_metadata(&video);
        assert_eq!(meta.raw_title.as_deref(), Some("Me at the zoo"));
        assert!(meta.genres.contains(&"film & animation".into()));
        assert!(meta.release_year.is_none());
        let mut unsupported = video;
        unsupported["acodec"] = Value::String("opus".into());
        assert_eq!(
            parse_stream(&unsupported).unwrap_err().code(),
            "unavailable"
        );
    }

    // @test RF-19
    #[test]
    fn relacionadas_removem_semente_e_repetidos() {
        let mut entries = parse_playlist_output(PLAYLIST).unwrap().entries;
        let seed = entries[0].video_id.clone();
        entries.push(entries[1].clone());
        let related = filter_related(entries, &seed, 50);
        assert_eq!(related.len(), 3);
        assert!(related.iter().all(|entry| entry.video_id != seed));
    }

    // @test RF-09
    #[test]
    fn parse_busca_ignora_linhas_ruins() {
        let r = parse_search_output(SEARCH);
        assert_eq!(r.len(), 2, "{r:?}");
        assert_eq!(r[0].video_id, "8_aIF8GvEK4");
        assert_eq!(r[0].channel, "Creative Commons Music");
        assert_eq!(r[0].duration_ms, Some(193_000));
        assert!(
            r[0].thumbnail_url
                .as_deref()
                .unwrap()
                .contains("hqdefault.jpg")
        );
        assert_eq!(parse_search_output("inválido\n{}\n").len(), 0);
    }

    // @test RF-09 RF-20
    #[test]
    fn reprocessa_fixtures_racionais_e_preserva_metadados_estruturados() {
        for (fixture, expected_title, expected_album) in [
            (
                include_str!("../tests/fixtures/racionais_q_xeCu3P9hU.json"),
                "Estilo Cachorro",
                "Nada Como Um Dia Após O Outro Dia (Ri Depois)",
            ),
            (
                include_str!("../tests/fixtures/racionais_fgysUhl98As.json"),
                "Eu sou 157",
                "Nada Como Um Dia Após O Outro Dia (Chora Agora)",
            ),
            (
                include_str!("../tests/fixtures/racionais_pL4bzW2qGqQ.json"),
                "Jorge da Capadócia",
                "Sobrevivendo no Inferno",
            ),
        ] {
            let v: Value = serde_json::from_str(fixture).unwrap();
            let track = corrected_track(&v);
            assert_eq!(track.title, expected_title);
            assert_eq!(track.artist, "Racionais");
            assert_eq!(track.album.as_deref(), Some(expected_album));
        }
        let structured = serde_json::json!({"title":"Racionais - álbum - cru", "channel":"Racionais - Topic", "track":"Estruturada", "artist":"Artista estruturado", "album":"Álbum estruturado", "release_year":2003});
        assert_eq!(
            parse_video_metadata(&structured).channel.as_deref(),
            Some("Racionais - Topic")
        );
        let track = corrected_track(&structured);
        assert_eq!(track.title, "Estruturada");
        assert_eq!(track.artist, "Artista estruturado");
        assert_eq!(track.album.as_deref(), Some("Álbum estruturado"));
        assert_eq!(track.release_year, Some(2003));

        let known_good: Value = serde_json::from_str(include_str!(
            "../tests/fixtures/ytdlp_daftpunk_get_lucky.json"
        ))
        .unwrap();
        let track = corrected_track(&known_good);
        assert_eq!(track.title, "Get Lucky");
        assert_eq!(
            track.artist,
            "Daft Punk feat. Pharrell Williams, Nile Rodgers"
        );
        assert_eq!(track.release_year, None);
    }

    // @test RF-09 RF-12
    #[test]
    fn campos_yt_dlp_respeitam_ordem_e_artistas_estruturados() {
        let reversed =
            serde_json::json!({"title":"NEGRO DRAMA - RACIONAIS MC'S", "channel":"Racionais TV"});
        let got = corrected_track(&reversed);
        assert_eq!(got.title, "NEGRO DRAMA");
        assert_eq!(got.artist, "RACIONAIS MC'S");
        assert_eq!(got.album, None);

        let structured = serde_json::json!({"title":"Vídeo", "channel":"Canal", "track":"Música", "artists":["Matuê", "X"]});
        let got = corrected_track(&structured);
        assert_eq!(got.title, "Música");
        assert_eq!(got.artist, "Matuê, X");
    }

    // @test RF-09
    #[test]
    fn filtro_rejeita_coletaneas_e_duracao_sem_rejeitar_faixa_mix() {
        let item = |title: &str, ms| SearchResult {
            video_id: "abcdefghijk".into(),
            title: title.into(),
            channel: "Artista".into(),
            duration_ms: ms,
            thumbnail_url: None,
        };
        for title in [
            "Artista - Full Album 2024",
            "Álbum completo de artista",
            "Greatest Hits",
            "Playlist de sucessos",
            "Coletânea 1 hora",
            "Rock 1-hour compilation",
            "Full-album collection",
            "Artista mix músicas",
        ] {
            assert!(!is_song_result(&item(title, Some(500_000))), "{title}");
        }
        assert!(!is_song_result(&item("Uma faixa", Some(600_001))));
        assert!(is_song_result(&item("Mix", Some(180_000))));
        assert!(is_song_result(&item(
            "Artista - Música (Radio Mix)",
            Some(210_000)
        )));
        assert!(is_song_result(&item(
            "Artista - Música (Extended Mix)",
            Some(400_000)
        )));
        assert!(!is_song_result(&item("Artista mix músicas", None)));
        assert!(is_song_result(&item("Artista - Música", Some(180_000))));
    }

    // @test RF-11
    #[test]
    fn parse_playlist() {
        let p = parse_playlist_output(PLAYLIST).unwrap();
        assert_eq!(p.id, "PLMC9KNkIncKtPzgY-5rmhvj7fax8fdxoj");
        assert!(p.title.as_deref().unwrap().contains("Pop Music"));
        assert_eq!(p.entries.len(), 4);
        assert!(parse_playlist_output("lixo").is_err());
    }

    // @test RF-10
    #[test]
    fn progresso() {
        let values: Vec<_> = PROGRESS.lines().filter_map(parse_progress_line).collect();
        assert_eq!(values.len(), 10);
        assert!(values.first().unwrap() < values.last().unwrap());
        assert_eq!(*values.last().unwrap(), 1.0);
        assert_eq!(
            parse_file_line(PROGRESS.lines().last().unwrap()),
            Some(PathBuf::from("/tmp/miupy-zoo.m4a"))
        );
        assert_eq!(parse_progress_line("MIUPYPROG 50 100 NA"), Some(0.5));
        assert_eq!(parse_progress_line("MIUPYPROG 50 NA 200"), Some(0.25));
        assert_eq!(parse_progress_line("MIUPYPROG 150 100 NA"), Some(1.0));
        assert_eq!(parse_progress_line("MIUPYPROG 10 NA NA"), None);
        assert_eq!(parse_progress_line("MIUPYPROG NA NA NA"), None);
        assert_eq!(parse_progress_line("MIUPYPROG 0 0 NA"), None);
        assert_eq!(parse_progress_line("[download] 10%"), None);
        assert_eq!(parse_progress_line(""), None);
    }

    // @test RF-10
    #[test]
    fn linha_do_arquivo_final() {
        assert_eq!(
            parse_file_line("MIUPYFILE\t/tmp/a b/x.m4a"),
            Some(PathBuf::from("/tmp/a b/x.m4a"))
        );
        assert_eq!(parse_file_line("MIUPYFILE\tNA"), None);
        assert_eq!(parse_file_line("outra coisa"), None);
    }

    // @test RF-09
    #[test]
    fn consulta_com_hifen_nao_vira_opcao() {
        let args = search_args("--exec rm -rf ~", 5).unwrap();
        assert!(args.contains(&"ytsearch5:--exec rm -rf ~".to_string()));
        assert!(!args.iter().any(|a| a == "--exec"));
        let last = args.last().unwrap();
        assert!(last.starts_with("ytsearch"));
    }

    // @test RF-09
    #[test]
    fn consulta_invalida_e_limite() {
        assert_eq!(search_args("   ", 5).unwrap_err().code(), "invalid_input");
        assert_eq!(search_args("a\nb", 5).unwrap_err().code(), "invalid_input");
        assert_eq!(
            search_args(&"x".repeat(1000), 5).unwrap_err().code(),
            "invalid_input"
        );
        assert!(
            search_args("a", 0)
                .unwrap()
                .last()
                .unwrap()
                .starts_with("ytsearch1:")
        );
        assert!(
            search_args("a", 9999)
                .unwrap()
                .last()
                .unwrap()
                .starts_with("ytsearch150:")
        );
    }

    // @test RF-10
    #[test]
    fn argumentos_de_download() {
        let args = download_args("dQw4w9WgXcQ", Path::new("/music"), "abc");
        let pos = |s: &str| args.iter().position(|a| a == s).unwrap();
        assert_eq!(args[pos("-f") + 1], "140/bestaudio[ext=m4a]");
        assert_eq!(args[pos("-o") + 1], "/music/abc.%(ext)s");
        assert_eq!(
            args[pos("--") + 1],
            "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
        );
        assert!(args.contains(&"--no-playlist".to_string()));
        assert!(
            !args
                .iter()
                .any(|a| a.contains("extract-audio") || a == "-x")
        );
    }

    // @test RF-10
    #[test]
    fn validacao_de_ids_e_nomes() {
        assert!(validate_video_id("dQw4w9WgXcQ").is_ok());
        assert!(validate_video_id("-abcdefghij").is_ok()); // id válido que começa com '-'
        assert!(validate_video_id("curto").is_err());
        assert!(validate_video_id("dQw4w9WgXc;").is_err());
        assert!(validate_file_stem("0192-abc").is_ok());
        assert!(validate_file_stem("../x").is_err());
        assert!(validate_file_stem("a/b").is_err());
        assert!(validate_file_stem(".oculto").is_err());
        assert!(validate_file_stem("a%(x)s").is_err());
        assert!(validate_file_stem("").is_err());
    }

    // @test RF-11
    #[test]
    fn id_de_playlist() {
        let id = "PLrAXtmErZgOeiKm4sgNOknGvNjby9efdf";
        assert_eq!(playlist_id(id).unwrap(), id);
        assert_eq!(
            playlist_id(&format!("https://www.youtube.com/playlist?list={id}")).unwrap(),
            id
        );
        assert_eq!(
            playlist_id(&format!("https://music.youtube.com/watch?v=abc&list={id}")).unwrap(),
            id
        );
        assert!(playlist_id("https://evil.example/playlist?list=PLrAXtmErZgOeiKm4").is_err());
        assert!(playlist_id("https://www.youtube.com/watch?v=abc").is_err());
        assert!(playlist_id("--version").is_err());
        assert!(playlist_id("").is_err());
        let args = playlist_args(id);
        assert_eq!(args[args.len() - 2], "--");
    }

    // @test RF-10
    #[test]
    fn classificacao_de_erros() {
        let c = |s: &str| classify_stderr(s).code();
        let errors: Value = serde_json::from_str(ERRORS).unwrap();
        assert_eq!(c(errors["indisponivel"].as_str().unwrap()), "not_found");
        assert_eq!(c(errors["formato"].as_str().unwrap()), "unavailable");
        assert_eq!(c(errors["mix"].as_str().unwrap()), "not_found");
        assert_eq!(c(errors["rede"].as_str().unwrap()), "network");
        assert_eq!(
            c("ERROR: This video is not available in your country"),
            "unavailable"
        );
        assert_eq!(c("ERROR: Sign in to confirm your age"), "unavailable");
        assert_eq!(c("ERROR: [youtube] x: Video unavailable"), "not_found");
        assert_eq!(c("ERROR: [youtube] x: Private video. Sign in"), "not_found");
        assert_eq!(
            c(
                "ERROR: Unable to download webpage: <urlopen error [Errno -3] Temporary failure in name resolution>"
            ),
            "network"
        );
        assert_eq!(c("ERROR: Requested format is not available"), "unavailable");
        assert_eq!(c("ERROR: algo estranho"), "internal");
        assert_eq!(c(""), "internal");
    }

    // @test RF-10
    #[test]
    fn nomes_parciais() {
        assert!(is_partial_name("abc", "abc.m4a.part"));
        assert!(is_partial_name("abc", "abc.m4a.ytdl"));
        assert!(is_partial_name("abc", "abc.f140.m4a"));
        assert!(!is_partial_name("abc", "abc.m4a"));
        assert!(!is_partial_name("abc", "abcd.m4a.part"));
        assert!(!is_partial_name("abc", "outro.m4a.part"));
    }

    // @test RF-09
    #[tokio::test]
    async fn binario_ausente_da_erro_unavailable_com_instrucoes() {
        let r = YtDlpResolver::with_binary("/nonexistent/yt-dlp-miupy");
        let e = r.check_available().await.unwrap_err();
        assert_eq!(e.code(), "unavailable");
        assert!(e.to_string().contains("pipx install yt-dlp"));
        let e = r.search("abc", 3).await.unwrap_err();
        assert_eq!(e.code(), "unavailable");
        let dir = tempfile::tempdir().unwrap();
        let e = r
            .download_audio(DownloadRequest::new("dQw4w9WgXcQ", dir.path(), "x"))
            .await
            .unwrap_err();
        assert_eq!(e.code(), "unavailable");
    }

    // Processo falso (script de shell) para exercitar o laço de leitura, progresso, cancelamento
    // e limpeza SEM yt-dlp. Não prova compatibilidade com o yt-dlp real.
    #[cfg(unix)]
    mod fake {
        use super::*;
        use std::os::unix::fs::PermissionsExt;
        use std::sync::{Arc, Mutex};

        fn fake_bin(dir: &Path, body: &str) -> PathBuf {
            let p = dir.join("fake-yt-dlp");
            std::fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
            p
        }

        // @test RF-10
        #[tokio::test]
        async fn cancelamento_mata_processo_e_limpa_parciais() {
            let tmp = tempfile::tempdir().unwrap();
            let out = tempfile::tempdir().unwrap();
            let stem = "faixa1";
            let bin = fake_bin(
                tmp.path(),
                &format!(
                    "echo 'MIUPYPROG 10 100 NA'\n: > '{0}/{1}.m4a.part'\nexec sleep 30",
                    out.path().display(),
                    stem
                ),
            );
            let seen = Arc::new(Mutex::new(Vec::new()));
            let seen2 = seen.clone();
            let cancel = tokio_util::sync::CancellationToken::new();
            let req = DownloadRequest::new("dQw4w9WgXcQ", out.path(), stem)
                .with_progress(Arc::new(move |p| seen2.lock().unwrap().push(p)))
                .with_cancel(cancel.clone());
            let r = YtDlpResolver::with_binary(bin);
            let c2 = cancel.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(500)).await;
                c2.cancel();
            });
            let started = std::time::Instant::now();
            let e = r.download_audio(req).await.unwrap_err();
            assert_eq!(e.code(), "cancelled");
            assert!(started.elapsed() < Duration::from_secs(10));
            assert_eq!(seen.lock().unwrap().first().copied(), Some(0.1));
            assert!(!out.path().join(format!("{stem}.m4a.part")).exists());
        }

        // @test RF-10 RF-26
        #[tokio::test]
        async fn spawn_repete_quando_o_executavel_esta_ocupado() {
            let mut calls = 0;
            let r = retry_text_busy(|| {
                calls += 1;
                if calls < 3 {
                    Err(std::io::Error::from_raw_os_error(ETXTBSY))
                } else {
                    Ok(calls)
                }
            })
            .await;
            assert_eq!(r.unwrap(), 3);

            let mut calls = 0;
            let r: std::io::Result<()> = retry_text_busy(|| {
                calls += 1;
                Err(std::io::Error::from(std::io::ErrorKind::NotFound))
            })
            .await;
            assert_eq!(r.unwrap_err().kind(), std::io::ErrorKind::NotFound);
            assert_eq!(calls, 1, "outros erros não são repetidos");

            let mut calls = 0;
            let r: std::io::Result<()> = retry_text_busy(|| {
                calls += 1;
                Err(std::io::Error::from_raw_os_error(ETXTBSY))
            })
            .await;
            assert_eq!(r.unwrap_err().raw_os_error(), Some(ETXTBSY));
            assert_eq!(calls, 6, "desiste depois de 5 repetições");
        }

        // @test RF-10
        #[tokio::test]
        async fn falha_do_processo_vira_erro_tipado_e_limpa() {
            let tmp = tempfile::tempdir().unwrap();
            let out = tempfile::tempdir().unwrap();
            let bin = fake_bin(
                tmp.path(),
                &format!(
                    ": > '{}/f2.m4a.part'\necho 'ERROR: [youtube] x: Video unavailable' >&2\nexit 1",
                    out.path().display()
                ),
            );
            let r = YtDlpResolver::with_binary(bin);
            let e = r
                .download_audio(DownloadRequest::new("dQw4w9WgXcQ", out.path(), "f2"))
                .await
                .unwrap_err();
            assert_eq!(e.code(), "not_found", "{e:?}");
            assert!(!out.path().join("f2.m4a.part").exists());
        }

        // @test RF-09
        #[tokio::test]
        async fn busca_via_processo_falso() {
            let tmp = tempfile::tempdir().unwrap();
            let fixture = tmp.path().join("out.jsonl");
            std::fs::write(&fixture, SEARCH).unwrap();
            let bin = fake_bin(tmp.path(), &format!("cat '{}'", fixture.display()));
            let r = YtDlpResolver::with_binary(bin);
            let res = r.search("qualquer", 5).await.unwrap();
            assert_eq!(
                res.len(),
                1,
                "o filtro removeu vídeo com mais de dez minutos"
            );
        }
    }
}
