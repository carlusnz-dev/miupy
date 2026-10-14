use miupy_core::{Track, TrackQuery};
use tauri::State;

use crate::downloads::remove_file_quietly;
use crate::error::AppError;
use crate::state::AppState;

type R<T> = Result<T, AppError>;

// @req RF-05 RF-08
#[tauri::command]
pub async fn library_list_tracks(
    state: State<'_, AppState>,
    query: Option<TrackQuery>,
) -> R<Vec<Track>> {
    let query = query.unwrap_or_default();
    state.db.run(move |db| Ok(db.list_tracks(&query)?)).await
}

// @req RF-05 RF-08
#[tauri::command]
pub async fn library_list_tracks_by_artist(
    state: State<'_, AppState>,
    artist_id: String,
    query: Option<TrackQuery>,
) -> R<Vec<Track>> {
    let query = query.unwrap_or_default();
    state
        .db
        .run(move |db| Ok(db.list_tracks_by_artist(&artist_id, &query)?))
        .await
}

// @req RF-05
#[tauri::command]
pub async fn library_get_track(state: State<'_, AppState>, track_id: String) -> R<Track> {
    state.db.run(move |db| Ok(db.get_track(&track_id)?)).await
}

// @req RF-04
#[tauri::command]
pub async fn library_set_favorite(
    state: State<'_, AppState>,
    track_id: String,
    favorite: bool,
) -> R<Track> {
    let track = state
        .db
        .run(move |db| Ok(db.set_favorite(&track_id, favorite)?))
        .await?;
    state.events.library_changed("favorite");
    Ok(track)
}

// @req RF-06
#[tauri::command]
pub async fn library_delete_track(
    state: State<'_, AppState>,
    track_id: String,
    delete_file: bool,
) -> R<()> {
    let id = track_id.clone();
    let (deleted, cover_shared) = state
        .db
        .run(move |db| {
            let deleted = db.delete_track(&id)?;
            let shared = deleted.cover_path.as_ref().is_some_and(|p| {
                db.list_tracks(&TrackQuery::default())
                    .map(|ts| ts.iter().any(|t| t.cover_path.as_ref() == Some(p)))
                    .unwrap_or(false)
            });
            Ok((deleted, shared))
        })
        .await?;
    state.player.remove_track(&track_id);
    if delete_file {
        let dirs = state.dirs.clone();
        // Só apagamos arquivos que estão no diretório de dados do app.
        tokio::task::spawn_blocking(move || {
            if let Some(p) = &deleted.file_path {
                remove_file_quietly(&dirs, std::path::Path::new(p));
            }
            if !cover_shared && let Some(p) = &deleted.cover_path {
                remove_file_quietly(&dirs, std::path::Path::new(p));
            }
        })
        .await
        .map_err(AppError::from)?;
    }
    state.events.library_changed("track_deleted");
    Ok(())
}
