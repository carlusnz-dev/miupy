//! Trait `MediaResolver`: busca e download de áudio de uma fonte (YouTube).
//!
//! Implementações: [`crate::ytdlp::YtDlpResolver`] (desktop, processo externo). Ponto de extensão
//! para `rustypipe` (Rust puro, Android): implementar esta trait num módulo atrás de uma feature
//! opcional. Ver o README da crate; a dependência NÃO foi adicionada (GPL-3.0, licença em aberto).

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use crate::error::Result;

/// Um vídeo encontrado numa busca ou numa playlist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResult {
    /// Id do vídeo no YouTube (11 caracteres).
    pub video_id: String,
    pub title: String,
    /// Nome do canal (para canais "Artista - Topic", é o artista).
    pub channel: String,
    /// Duração em ms, quando a fonte informa (transmissões ao vivo não têm).
    pub duration_ms: Option<u64>,
    pub thumbnail_url: Option<String>,
}

/// Playlist resolvida: título e vídeos, na ordem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistInfo {
    pub id: String,
    pub title: Option<String>,
    pub entries: Vec<SearchResult>,
}

/// Callback de progresso: recebe um valor em `0.0..=1.0`. Chamado da tarefa de download, então
/// deve ser barato e não bloquear.
pub type ProgressFn = Arc<dyn Fn(f32) + Send + Sync>;

/// Pedido de download de áudio.
#[derive(Clone)]
pub struct DownloadRequest {
    pub video_id: String,
    /// Diretório de destino (criado se não existir).
    pub dest_dir: PathBuf,
    /// Nome do arquivo sem extensão (ex.: o id da faixa). Sem separadores de caminho.
    pub file_stem: String,
    /// Progresso 0..1, opcional.
    pub progress: Option<ProgressFn>,
    /// Cancelar este token mata o processo/transferência e apaga arquivos parciais.
    pub cancel: CancellationToken,
}

impl DownloadRequest {
    pub fn new(
        video_id: impl Into<String>,
        dest_dir: impl Into<PathBuf>,
        file_stem: impl Into<String>,
    ) -> Self {
        Self {
            video_id: video_id.into(),
            dest_dir: dest_dir.into(),
            file_stem: file_stem.into(),
            progress: None,
            cancel: CancellationToken::new(),
        }
    }

    pub fn with_progress(mut self, progress: ProgressFn) -> Self {
        self.progress = Some(progress);
        self
    }

    pub fn with_cancel(mut self, cancel: CancellationToken) -> Self {
        self.cancel = cancel;
        self
    }
}

/// Áudio baixado, com propriedades lidas do arquivo final.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadedAudio {
    pub path: PathBuf,
    /// "aac" para M4A; ver [`crate::tagger::AudioInfo::codec`].
    pub codec: String,
    /// kbps, se o arquivo informar.
    pub bitrate_kbps: Option<u32>,
    pub duration_ms: u64,
}

/// URL temporária de áudio M4A/AAC para reprodução sem download.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamAudio {
    pub url: String,
    pub http_headers: BTreeMap<String, String>,
    pub codec: String,
    pub container: String,
    pub bitrate_kbps: Option<u32>,
    pub duration_ms: Option<u64>,
    /// Instante UTC, em segundos Unix, se a URL informa `expire`.
    pub expires_at: Option<u64>,
}

/// Metadados secundários fornecidos pelo vídeo, antes do enriquecimento pelo MusicBrainz.
// @req RF-20
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VideoMetadata {
    pub channel: Option<String>,
    pub raw_title: Option<String>,
    pub artist: Option<String>,
    pub track: Option<String>,
    pub album: Option<String>,
    pub release_year: Option<u32>,
    pub genres: Vec<String>,
}

/// Fonte de busca e download de áudio. Trocável (yt-dlp, rustypipe...).
#[async_trait]
pub trait MediaResolver: Send + Sync {
    /// Busca vídeos; `limit` é limitado a 1..=50.
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>>;

    /// Baixa o áudio (M4A/AAC) de um vídeo.
    async fn download_audio(&self, request: DownloadRequest) -> Result<DownloadedAudio>;

    /// Lista os vídeos de uma playlist (URL ou id).
    async fn resolve_playlist(&self, url_or_id: &str) -> Result<PlaylistInfo>;

    // @req RF-18
    /// Resolve uma URL direta e temporária de áudio M4A/AAC.
    async fn resolve_stream(&self, video_id: &str) -> Result<StreamAudio>;

    // @req RF-19
    /// Busca vídeos no mix de um vídeo. Mix indisponível devolve lista vazia.
    async fn related_tracks(&self, video_id: &str, limit: usize) -> Result<Vec<SearchResult>>;

    // @req RF-20
    /// Lê campos musicais fornecidos pelo vídeo como fonte secundária.
    async fn video_metadata(&self, video_id: &str) -> Result<VideoMetadata>;
}
