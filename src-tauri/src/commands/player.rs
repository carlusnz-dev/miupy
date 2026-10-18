use tauri::State;

use crate::db::blocking;
use crate::dto::{PlayerState, QueueContext};
use crate::error::AppError;
use crate::state::AppState;
use miupy_core::ShuffleMode;
use miupy_core::Track;

type R<T> = Result<T, AppError>;

// @req RF-01
#[tauri::command]
pub async fn player_get_state(state: State<'_, AppState>) -> R<PlayerState> {
    let p = state.player.clone();
    blocking(move || Ok(p.state())).await
}

// @req RF-03
#[tauri::command]
pub async fn player_get_queue(state: State<'_, AppState>) -> R<Vec<Track>> {
    let p = state.player.clone();
    blocking(move || p.queue_tracks()).await
}

// @req RF-49
#[tauri::command]
pub async fn player_enqueue(
    state: State<'_, AppState>,
    track_ids: Vec<String>,
    next: Option<bool>,
) -> R<PlayerState> {
    let p = state.player.clone();
    blocking(move || p.enqueue(track_ids, next.unwrap_or(false))).await
}

// @req RF-49
#[tauri::command]
pub async fn player_queue_remove(state: State<'_, AppState>, index: i64) -> R<PlayerState> {
    let index = usize::try_from(index).map_err(|_| AppError::invalid("posição inválida"))?;
    let p = state.player.clone();
    blocking(move || p.queue_remove(index)).await
}

// @req RF-49
#[tauri::command]
pub async fn player_queue_move(state: State<'_, AppState>, from: i64, to: i64) -> R<PlayerState> {
    let from = usize::try_from(from).map_err(|_| AppError::invalid("posição inválida"))?;
    let to = usize::try_from(to).map_err(|_| AppError::invalid("posição inválida"))?;
    let p = state.player.clone();
    blocking(move || p.queue_move(from, to)).await
}

// @req RF-01 RF-03 RF-42
#[tauri::command]
pub async fn player_play(
    state: State<'_, AppState>,
    track_id: String,
    queue: Option<Vec<String>>,
    context: Option<QueueContext>,
) -> R<PlayerState> {
    let p = state.player.clone();
    blocking(move || p.play_with_context(&track_id, queue, context)).await
}

// @req RF-01
#[tauri::command]
pub async fn player_pause(state: State<'_, AppState>) -> R<PlayerState> {
    let p = state.player.clone();
    blocking(move || p.pause()).await
}

// @req RF-01
#[tauri::command]
pub async fn player_resume(state: State<'_, AppState>) -> R<PlayerState> {
    let p = state.player.clone();
    blocking(move || p.resume()).await
}

// @req RF-01
#[tauri::command]
pub async fn player_stop(state: State<'_, AppState>) -> R<PlayerState> {
    let p = state.player.clone();
    blocking(move || p.stop()).await
}

// @req RF-03
#[tauri::command]
pub async fn player_seek(state: State<'_, AppState>, position_ms: u64) -> R<PlayerState> {
    let p = state.player.clone();
    blocking(move || p.seek(position_ms)).await
}

// @req RF-03
#[tauri::command]
pub async fn player_next(state: State<'_, AppState>) -> R<PlayerState> {
    let p = state.player.clone();
    blocking(move || p.next()).await
}

// @req RF-03
#[tauri::command]
pub async fn player_previous(state: State<'_, AppState>) -> R<PlayerState> {
    let p = state.player.clone();
    blocking(move || p.previous()).await
}

// @req RF-02
#[tauri::command]
pub async fn player_set_volume(state: State<'_, AppState>, volume: f64) -> R<PlayerState> {
    let p = state.player.clone();
    blocking(move || p.set_volume(volume)).await
}

// @req RF-02
#[tauri::command]
pub async fn player_set_muted(state: State<'_, AppState>, muted: bool) -> R<PlayerState> {
    let p = state.player.clone();
    blocking(move || p.set_muted(muted)).await
}

// @req RF-24
#[tauri::command]
pub async fn player_set_shuffle(state: State<'_, AppState>, mode: ShuffleMode) -> R<PlayerState> {
    let p = state.player.clone();
    blocking(move || p.set_shuffle(mode)).await
}
