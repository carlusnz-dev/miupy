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
    let deleted = state.db.run(move |db| Ok(db.delete_track(&id)?)).await?;
    state.player.remove_track(&track_id);
    if delete_file {
        let dirs = state.dirs.clone();
        // Só apagamos arquivos que estão no diretório de dados do app.
        tokio::task::spawn_blocking(move || {
            for p in [&deleted.file_path, &deleted.cover_path]
                .into_iter()
                .flatten()
            {
                remove_file_quietly(&dirs, std::path::Path::new(p));
            }
        })
        .await
        .map_err(AppError::from)?;
    }
    state.events.library_changed("track_deleted");
    Ok(())
}
