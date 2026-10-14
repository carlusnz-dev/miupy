use miupy_core::Track;
use tauri::State;

use crate::dto::SearchResult;
use crate::error::AppError;
use crate::state::AppState;

type R<T> = Result<T, AppError>;

// @req RF-09
#[tauri::command]
pub async fn youtube_search(
    state: State<'_, AppState>,
    query: String,
    limit: Option<u32>,
) -> R<Vec<SearchResult>> {
    state.youtube.search(&query, limit).await
}

// @req RF-09
#[tauri::command]
pub async fn youtube_add_to_library(
    state: State<'_, AppState>,
    source_ids: Vec<String>,
    playlist_id: Option<String>,
) -> R<Vec<Track>> {
    state.youtube.add_to_library(source_ids, playlist_id).await
}
