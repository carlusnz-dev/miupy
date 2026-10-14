use miupy_core::{Dna, LibraryStats, TasteProfile, TrackScore, Wrapped};
use tauri::State;

use crate::error::AppError;
use crate::state::AppState;
use crate::tz::local_offset_minutes;

type R<T> = Result<T, AppError>;

// @req RF-14
#[tauri::command]
pub async fn stats_library(state: State<'_, AppState>) -> R<LibraryStats> {
    state.db.run(|db| Ok(db.library_stats()?)).await
}

// @req RF-15
#[tauri::command]
pub async fn stats_wrapped(state: State<'_, AppState>, year: i32) -> R<Wrapped> {
    // O fuso é do relógio do sistema, calculado aqui (o núcleo não o lê).
    let tz = local_offset_minutes();
    state.db.run(move |db| Ok(db.wrapped(year, tz)?)).await
}

// @req RF-15
#[tauri::command]
pub async fn stats_wrapped_years(state: State<'_, AppState>) -> R<Vec<i32>> {
    let tz = local_offset_minutes();
    state.db.run(move |db| Ok(db.wrapped_years(tz)?)).await
}

// @req RF-19
#[tauri::command]
pub async fn stats_taste(state: State<'_, AppState>) -> R<TasteProfile> {
    state.db.run(|db| Ok(db.taste_profile(None)?)).await
}

// @req RF-25
#[tauri::command]
pub async fn stats_track_scores(
    state: State<'_, AppState>,
    track_ids: Option<Vec<String>>,
) -> R<Vec<TrackScore>> {
    state
        .db
        .run(move |db| Ok(db.track_scores(track_ids.as_deref())?))
        .await
}

// @req RF-25
#[tauri::command]
pub async fn stats_dna(state: State<'_, AppState>, window_days: Option<u32>) -> R<Dna> {
    let now = chrono::Utc::now().timestamp_millis();
    state.db.run(move |db| Ok(db.dna(window_days, now)?)).await
}
