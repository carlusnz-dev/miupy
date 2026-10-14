//! `miupy-media`: tudo que fala com a internet ou mexe no arquivo de mídia.
//!
//! - [`resolver`]: trait [`MediaResolver`] (busca e download de áudio) e tipos.
//! - [`ytdlp`]: implementação por processo `yt-dlp`.
//! - [`metadata`]: [`MetadataProvider`] (MusicBrainz, Cover Art Archive, LRCLIB).
//! - [`tagger`]: leitura/escrita de tags e capa com `lofty`.
//! - [`title`]: limpeza de títulos de vídeo.
//!
//! Esta crate não depende de Tauri nem das outras crates do workspace.

pub mod error;
pub mod metadata;
pub mod ratelimit;
pub mod resolver;
pub mod suggest;
pub mod tagger;
pub mod title;
pub mod ytdlp;
pub mod ytdlp_manager;

pub use error::{MediaError, Result};
pub use metadata::{
    ArtistDetails, ArtistPhoto, LyricsData, MetadataConfig, MetadataProvider, OnlineMetadata,
    RecordingMatch,
};
pub use resolver::{
    DownloadRequest, DownloadedAudio, MediaResolver, PlaylistInfo, ProgressFn, SearchResult,
    StreamAudio, VideoMetadata,
};
pub use suggest::suggest;
pub use tagger::{AudioInfo, Cover, TagUpdate};
pub use title::{ParsedTitle, clean_title};
pub use ytdlp::{CorrectedTrack, YtDlpResolver, corrected_track, is_song_result};
pub use ytdlp_manager::{YtDlpManager, YtDlpState, YtDlpStatus, YtDlpStatusFn};
