use miupy_core::{ArtistInfo, Lyrics, Track};
use tauri::State;

use crate::dto::MetadataRefreshStatus;
use crate::error::AppError;
use crate::state::AppState;

type R<T> = Result<T, AppError>;

// @req RF-12
#[tauri::command]
pub async fn metadata_refresh_track(state: State<'_, AppState>, track_id: String) -> R<Track> {
    state.metadata.refresh_track(&track_id).await
}

// @req RF-12
#[tauri::command]
pub async fn metadata_get_artist(state: State<'_, AppState>, artist_id: String) -> R<ArtistInfo> {
    state.metadata.get_artist(&artist_id).await
}

// @req RF-12
#[tauri::command]
pub async fn metadata_get_lyrics(state: State<'_, AppState>, track_id: String) -> R<Lyrics> {
    state.metadata.get_lyrics(&track_id).await
}

// @req RF-12 RF-20
#[tauri::command]
pub async fn metadata_refresh_library(
    state: State<'_, AppState>,
    only_missing: Option<bool>,
) -> R<usize> {
    state
        .metadata
        .enqueue_library_refresh(only_missing.unwrap_or(true))
}

// @req RF-32
#[tauri::command]
pub fn metadata_refresh_status(state: State<'_, AppState>) -> MetadataRefreshStatus {
    state.metadata.refresh_status()
}

// @req RF-32
#[tauri::command]
pub fn metadata_refresh_cancel(state: State<'_, AppState>) {
    state.metadata.cancel_library_refresh();
}
