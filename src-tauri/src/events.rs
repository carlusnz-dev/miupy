//! Saída de eventos para o frontend. A lógica de orquestração fala com o trait [`Events`];
//! só [`TauriEvents`] conhece o Tauri, o que deixa o resto testável sem janela.

use miupy_core::DownloadJob;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::dto::{MetadataRefreshStatus, PlayerState, Progress, YtDlpStatus};

pub const EV_PLAYER_STATE: &str = "player://state";
pub const EV_PLAYER_PROGRESS: &str = "player://progress";
pub const EV_PLAYER_ERROR: &str = "player://error";
pub const EV_DOWNLOAD_PROGRESS: &str = "download://progress";
pub const EV_LIBRARY_CHANGED: &str = "library://changed";
// @req RF-19
pub const EV_RECOMMENDATIONS_CHANGED: &str = "recommendations://changed";

pub trait Events: Send + Sync + 'static {
    fn player_state(&self, state: &PlayerState);
    fn player_progress(&self, progress: Progress);
    fn player_error(&self, track_id: &str, code: &str, message: &str);
    fn ytdlp_status(&self, status: &YtDlpStatus);
    fn metadata_progress(&self, status: &MetadataRefreshStatus);
    fn download_progress(&self, job: &DownloadJob);
    fn library_changed(&self, reason: &str);
    fn recommendations_changed(&self, count: usize);
}

#[derive(Serialize, Clone)]
struct LibraryChanged<'a> {
    reason: &'a str,
}

pub struct TauriEvents(pub AppHandle);

impl TauriEvents {
    fn emit<S: Serialize + Clone>(&self, event: &str, payload: S) {
        if let Err(e) = self.0.emit(event, payload) {
            log::warn!("falha ao emitir {event}: {e}");
        }
    }
}

impl Events for TauriEvents {
    fn player_state(&self, state: &PlayerState) {
        self.emit(EV_PLAYER_STATE, state);
    }
    fn player_progress(&self, progress: Progress) {
        self.emit(EV_PLAYER_PROGRESS, progress);
    }
    fn player_error(&self, track_id: &str, code: &str, message: &str) {
        self.emit(
            EV_PLAYER_ERROR,
            serde_json::json!({"trackId":track_id,"code":code,"message":message}),
        );
    }
    fn ytdlp_status(&self, status: &YtDlpStatus) {
        self.emit("ytdlp://status", status);
    }
    // @req RF-32
    fn metadata_progress(&self, status: &MetadataRefreshStatus) {
        self.emit("metadata://progress", status);
    }
    fn download_progress(&self, job: &DownloadJob) {
        self.emit(EV_DOWNLOAD_PROGRESS, job);
    }
    fn library_changed(&self, reason: &str) {
        self.emit(EV_LIBRARY_CHANGED, LibraryChanged { reason });
    }
    fn recommendations_changed(&self, count: usize) {
        self.emit(
            EV_RECOMMENDATIONS_CHANGED,
            serde_json::json!({ "count": count }),
        );
    }
}

#[cfg(test)]
pub mod testing {
    use super::*;
    use std::sync::Mutex;

    /// Guarda tudo o que foi emitido, para os testes conferirem.
    #[derive(Default)]
    pub struct RecordingEvents {
        pub states: Mutex<Vec<PlayerState>>,
        pub player_errors: Mutex<Vec<(String, String, String)>>,
        pub progress: Mutex<Vec<Progress>>,
        pub downloads: Mutex<Vec<DownloadJob>>,
        pub library: Mutex<Vec<String>>,
        pub recommendations: Mutex<Vec<usize>>,
        pub metadata: Mutex<Vec<MetadataRefreshStatus>>,
    }

    impl Events for RecordingEvents {
        fn player_state(&self, state: &PlayerState) {
            self.states.lock().unwrap().push(state.clone());
        }
        fn player_progress(&self, progress: Progress) {
            self.progress.lock().unwrap().push(progress);
        }
        fn player_error(&self, track_id: &str, code: &str, message: &str) {
            self.player_errors
                .lock()
                .unwrap()
                .push((track_id.into(), code.into(), message.into()));
        }
        fn ytdlp_status(&self, _status: &YtDlpStatus) {}
        fn metadata_progress(&self, status: &MetadataRefreshStatus) {
            self.metadata.lock().unwrap().push(status.clone());
        }
        fn download_progress(&self, job: &DownloadJob) {
            self.downloads.lock().unwrap().push(job.clone());
        }
        fn library_changed(&self, reason: &str) {
            self.library.lock().unwrap().push(reason.to_owned());
        }
        fn recommendations_changed(&self, count: usize) {
            self.recommendations.lock().unwrap().push(count);
        }
    }
}
