//! Resolvedor Android sem processos externos.

use std::collections::{BTreeMap, HashSet};

use async_trait::async_trait;
use rustypipe::client::RustyPipe;
use rustypipe::model::{AudioFormat, VideoItem};
use tokio::io::AsyncWriteExt;

use crate::error::{MediaError, Result};
use crate::resolver::{
    DownloadRequest, DownloadedAudio, MediaResolver, PlaylistInfo, SearchResult, StreamAudio,
    VideoMetadata,
};
use crate::title::clean_title;
use crate::ytdlp::{is_song_result, playlist_id, validate_file_stem, validate_video_id};

/// Tamanho de cada pedido do download (o googlevideo aceita até ~10 MB sem limitar).
const CHUNK_BYTES: u64 = 10 * 1024 * 1024;
/// Páginas no máximo por busca ou mix quando o filtro de músicas descarta muito.
const MAX_PAGES: usize = 5;

pub struct RustyPipeResolver {
    client: RustyPipe,
    http: reqwest::Client,
}

impl RustyPipeResolver {
    pub fn new() -> Result<Self> {
        let client = RustyPipe::builder()
            .no_storage()
            .no_botguard()
            .build()
            .map_err(|e| MediaError::Internal(format!("RustyPipe: {e}")))?;
        let http = crate::http::client_builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| MediaError::Internal(format!("cliente HTTP: {e}")))?;
        Ok(Self { client, http })
    }

    /// Título e canal crus, como o yt-dlp: a casca limpa depois com os artistas conhecidos.
    fn result(video: VideoItem) -> SearchResult {
        let thumbnail_url = video
            .thumbnail
            .into_iter()
            .max_by_key(|t| t.width)
            .map(|t| t.url)
            .or_else(|| Some(format!("https://i.ytimg.com/vi/{}/hqdefault.jpg", video.id)));
        SearchResult {
            title: video.name,
            channel: video.channel.map(|c| c.name).unwrap_or_default(),
            duration_ms: video.duration.map(|d| u64::from(d) * 1000),
            thumbnail_url,
            video_id: video.id,
        }
    }

    fn network(e: impl std::fmt::Display) -> MediaError {
        MediaError::Network(e.to_string())
    }

    async fn page_items(
        &self,
        mut page: rustypipe::model::paginator::Paginator<VideoItem>,
        limit: usize,
        songs_only: bool,
    ) -> Result<Vec<SearchResult>> {
        let mut result = Vec::new();
        let mut seen = HashSet::new();
        // Busca e mix param em poucas páginas; playlist vai até o fim.
        let mut pages_left = if songs_only { MAX_PAGES } else { usize::MAX };
        loop {
            for video in &page.items {
                let entry = Self::result(video.clone());
                if seen.insert(entry.video_id.clone()) && (!songs_only || is_song_result(&entry)) {
                    result.push(entry);
                    if result.len() >= limit {
                        return Ok(result);
                    }
                }
            }
            pages_left -= 1;
            if pages_left == 0 {
                break;
            }
            page = match page
                .next(self.client.query())
                .await
                .map_err(Self::network)?
            {
                Some(next) => next,
                None => break,
            };
        }
        Ok(result)
    }
}

#[async_trait]
impl MediaResolver for RustyPipeResolver {
    // @req RF-09 RF-45
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let query = query.trim();
        if query.is_empty() || query.chars().count() > 300 || query.chars().any(char::is_control) {
            return Err(MediaError::InvalidInput(
                "consulta vazia ou inválida".into(),
            ));
        }
        let found = self
            .client
            .query()
            .search::<VideoItem, _>(query)
            .await
            .map_err(Self::network)?;
        self.page_items(found.items, limit.clamp(1, 50), true).await
    }

    // @req RF-10 RF-45
    async fn download_audio(&self, request: DownloadRequest) -> Result<DownloadedAudio> {
        validate_video_id(&request.video_id)?;
        validate_file_stem(&request.file_stem)?;
        tokio::fs::create_dir_all(&request.dest_dir).await?;
        let stream = self.resolve_stream(&request.video_id).await?;
        let path = request.dest_dir.join(format!("{}.m4a", request.file_stem));
        let partial = request
            .dest_dir
            .join(format!("{}.m4a.part", request.file_stem));
        let transfer = async {
            let user_agent = stream
                .http_headers
                .get("User-Agent")
                .map(String::as_str)
                .unwrap_or("Mozilla/5.0");
            // O googlevideo recusa (403) o arquivo inteiro num pedido só: baixa em pedaços pelo
            // parâmetro `range`, como o yt-dlp, com o tamanho do `clen` da própria URL.
            let total = reqwest::Url::parse(&stream.url).ok().and_then(|url| {
                url.query_pairs()
                    .find(|(key, _)| key == "clen")
                    .and_then(|(_, val)| val.parse::<u64>().ok())
            });
            let mut file = tokio::fs::File::create(&partial).await?;
            let mut received = 0_u64;
            loop {
                let end = received + CHUNK_BYTES - 1;
                let response = self
                    .http
                    .get(format!("{}&range={received}-{end}", stream.url))
                    .header("User-Agent", user_agent)
                    .send()
                    .await?;
                if response.status() == reqwest::StatusCode::FORBIDDEN {
                    // Sem PO token o googlevideo só entrega o começo do arquivo.
                    return Err(MediaError::Unavailable(
                        "O YouTube recusou o download sem o token de verificação.".into(),
                    ));
                }
                let mut response = response.error_for_status()?;
                let before = received;
                while let Some(chunk) = response.chunk().await? {
                    file.write_all(&chunk).await?;
                    received += chunk.len() as u64;
                    if let (Some(total), Some(progress)) = (total, &request.progress) {
                        progress((received as f64 / total as f64).clamp(0.0, 1.0) as f32);
                    }
                }
                let done = match total {
                    Some(total) => received >= total,
                    None => received - before < CHUNK_BYTES,
                };
                if done || received == before {
                    break;
                }
            }
            if total.is_some_and(|total| received < total) {
                return Err(MediaError::Network(
                    "download interrompido antes do fim".into(),
                ));
            }
            file.flush().await?;
            let check = partial.clone();
            let info = tokio::task::spawn_blocking(move || crate::tagger::read_tags(&check))
                .await
                .map_err(|e| MediaError::Internal(e.to_string()))??;
            if info.codec != "aac" {
                return Err(MediaError::Unavailable("áudio não é M4A/AAC".into()));
            }
            tokio::fs::rename(&partial, &path).await?;
            if let Some(progress) = &request.progress {
                progress(1.0);
            }
            Ok(DownloadedAudio {
                path,
                codec: info.codec,
                bitrate_kbps: info.bitrate_kbps,
                duration_ms: info.duration_ms,
            })
        };
        let outcome = tokio::select! {
            _ = request.cancel.cancelled() => Err(MediaError::Cancelled),
            value = transfer => value,
        };
        if outcome.is_err() {
            let _ = tokio::fs::remove_file(&partial).await;
        }
        outcome
    }

    // @req RF-11 RF-45
    async fn resolve_playlist(&self, url_or_id: &str) -> Result<PlaylistInfo> {
        let id = playlist_id(url_or_id)?;
        let playlist = self
            .client
            .query()
            .playlist(&id)
            .await
            .map_err(Self::network)?;
        let entries = self.page_items(playlist.videos, usize::MAX, false).await?;
        Ok(PlaylistInfo {
            id,
            title: Some(playlist.name),
            entries,
        })
    }

    // @req RF-18 RF-45
    async fn resolve_stream(&self, video_id: &str) -> Result<StreamAudio> {
        validate_video_id(video_id)?;
        let player = self
            .client
            .query()
            .player(video_id)
            .await
            .map_err(Self::network)?;
        let audio = player
            .audio_streams
            .iter()
            .filter(|s| s.format == AudioFormat::M4a && s.drm_track_type.is_none())
            .max_by_key(|s| (s.itag == 140, s.bitrate))
            .ok_or_else(|| MediaError::Unavailable("formato M4A/AAC indisponível".into()))?;
        if !audio.url.starts_with("https://") {
            return Err(MediaError::Unavailable("URL de áudio sem HTTPS".into()));
        }
        let expires_at = reqwest::Url::parse(&audio.url).ok().and_then(|url| {
            url.query_pairs()
                .find(|(key, _)| key == "expire")
                .and_then(|(_, val)| val.parse().ok())
        });
        Ok(StreamAudio {
            url: audio.url.clone(),
            http_headers: BTreeMap::from([(
                "User-Agent".into(),
                self.client
                    .query()
                    .user_agent(player.client_type)
                    .into_owned(),
            )]),
            codec: "aac".into(),
            container: "m4a".into(),
            bitrate_kbps: Some(audio.bitrate / 1000),
            duration_ms: audio
                .duration_ms
                .map(u64::from)
                .or(Some(u64::from(player.details.duration) * 1000)),
            expires_at,
        })
    }

    // @req RF-19 RF-45
    async fn related_tracks(&self, video_id: &str, limit: usize) -> Result<Vec<SearchResult>> {
        validate_video_id(video_id)?;
        let details = self.client.query().video_details(video_id).await;
        // Como no yt-dlp: mix de vídeo indisponível ou inexistente é lista vazia, não erro.
        let details = match details {
            Err(rustypipe::error::Error::Extraction(
                rustypipe::error::ExtractionError::Unavailable { .. }
                | rustypipe::error::ExtractionError::NotFound { .. },
            )) => return Ok(Vec::new()),
            other => other.map_err(Self::network)?,
        };
        let mut tracks = self
            .page_items(details.recommended, limit.clamp(1, 50) + 1, true)
            .await?;
        tracks.retain(|track| track.video_id != video_id);
        tracks.truncate(limit.clamp(1, 50));
        Ok(tracks)
    }

    // @req RF-20 RF-45
    async fn video_metadata(&self, video_id: &str) -> Result<VideoMetadata> {
        validate_video_id(video_id)?;
        let details = self
            .client
            .query()
            .video_details(video_id)
            .await
            .map_err(Self::network)?;
        let parsed = clean_title(&details.name, &details.channel.name);
        Ok(VideoMetadata {
            channel: Some(details.channel.name),
            raw_title: Some(details.name),
            artist: parsed.artist_credit(),
            track: Some(parsed.title),
            album: None,
            release_year: details.publish_date.map(|d| d.year() as u32),
            genres: Vec::new(),
        })
    }
}
