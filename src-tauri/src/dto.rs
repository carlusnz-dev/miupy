//! Tipos que só existem no contrato de IPC (o resto vem direto de `miupy-core`, que já
//! serializa em camelCase).

use miupy_audio::PlaybackStatus;
use miupy_core::{ShuffleMode, Track};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PlayerStatus {
    Stopped,
    Loading,
    Playing,
    Paused,
}

impl From<PlaybackStatus> for PlayerStatus {
    fn from(s: PlaybackStatus) -> Self {
        match s {
            PlaybackStatus::Stopped => PlayerStatus::Stopped,
            PlaybackStatus::Loading => PlayerStatus::Loading,
            PlaybackStatus::Playing => PlayerStatus::Playing,
            PlaybackStatus::Paused => PlayerStatus::Paused,
        }
    }
}

/// `PlayerState` de `ipc.md`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerState {
    pub status: PlayerStatus,
    pub track: Option<Track>,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub volume: f32,
    pub muted: bool,
    pub queue: Vec<String>,
    /// -1 se a fila está vazia.
    pub queue_index: i64,
    // @req RF-18
    pub streaming: bool,
    pub shuffle: ShuffleMode,
}

/// Payload de `player://progress`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub position_ms: u64,
    pub duration_ms: u64,
}

/// `SearchResult` de `ipc.md`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub source_id: String,
    pub title: String,
    pub artist: String,
    pub duration_ms: u64,
    pub thumbnail_url: Option<String>,
    pub in_library: bool,
    // @req RF-04
    pub track_id: Option<String>,
    pub is_favorite: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageUpload {
    pub data_base64: String,
    pub mime_type: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum YtDlpState {
    Missing,
    Downloading,
    Ready,
    Error,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct YtDlpStatus {
    pub state: YtDlpState,
    pub version: Option<String>,
    pub path: Option<String>,
    pub checked_at: Option<u64>,
    pub progress: f32,
    pub error: Option<String>,
}
impl From<miupy_media::YtDlpStatus> for YtDlpStatus {
    fn from(s: miupy_media::YtDlpStatus) -> Self {
        Self {
            state: match s.state {
                miupy_media::YtDlpState::Missing => YtDlpState::Missing,
                miupy_media::YtDlpState::Downloading => YtDlpState::Downloading,
                miupy_media::YtDlpState::Ready => YtDlpState::Ready,
                miupy_media::YtDlpState::Error => YtDlpState::Error,
            },
            version: s.version,
            path: s.path.map(|p| p.to_string_lossy().into_owned()),
            checked_at: s.checked_at,
            progress: s.progress,
            error: s.error,
        }
    }
}
pub fn from_ytdlp_status(s: miupy_media::YtDlpStatus) -> YtDlpStatus {
    s.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    // @test RF-01
    #[test]
    fn player_state_serializa_como_o_contrato() {
        let s = PlayerState {
            status: PlayerStatus::Playing,
            track: None,
            position_ms: 1500,
            duration_ms: 200_000,
            volume: 0.8,
            muted: false,
            queue: vec!["a".into()],
            queue_index: 0,
            streaming: false,
            shuffle: ShuffleMode::Off,
        };
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["status"], "playing");
        assert_eq!(v["positionMs"], 1500);
        assert_eq!(v["durationMs"], 200_000);
        assert_eq!(v["queueIndex"], 0);
        assert!(v["track"].is_null());
        assert_eq!(v["queue"][0], "a");
        assert_eq!(serde_json::to_string(&s.volume).unwrap(), "0.8");
    }

    // @test RF-09
    #[test]
    fn search_result_serializa_em_camel_case() {
        let r = SearchResult {
            source_id: "abc".into(),
            title: "t".into(),
            artist: "a".into(),
            duration_ms: 10,
            thumbnail_url: None,
            in_library: true,
            track_id: Some("track".into()),
            is_favorite: true,
        };
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["sourceId"], "abc");
        assert_eq!(v["inLibrary"], true);
        assert!(v["thumbnailUrl"].is_null());
    }
}
