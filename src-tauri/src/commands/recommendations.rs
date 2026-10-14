use miupy_core::Recommendation;
use tauri::State;

use crate::error::AppError;
use crate::state::AppState;

type R<T> = Result<T, AppError>;

// @req RF-19
#[tauri::command]
pub async fn recommendations_list(
    state: State<'_, AppState>,
    playlist_id: Option<String>,
    limit: Option<u32>,
) -> R<Vec<Recommendation>> {
    state.recommendations.list(playlist_id, limit).await
}

// @req RF-19
#[tauri::command]
pub async fn recommendations_refresh(
    state: State<'_, AppState>,
    playlist_id: Option<String>,
) -> R<Vec<Recommendation>> {
    state.recommendations.refresh(playlist_id).await
}

// @req RF-19
#[tauri::command]
pub async fn recommendations_dismiss(state: State<'_, AppState>, source_id: String) -> R<()> {
    state.recommendations.dismiss(source_id).await
}
