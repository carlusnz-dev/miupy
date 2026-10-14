use miupy_core::{ArtistInfo, Lyrics, Track};
use tauri::State;

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
