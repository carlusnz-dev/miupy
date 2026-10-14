use miupy_core::{Playlist, PlaylistPatch, Track, TrackQuery};
use tauri::State;

use crate::error::AppError;
use crate::state::AppState;

type R<T> = Result<T, AppError>;

// @req RF-06
#[tauri::command]
pub async fn playlist_list(state: State<'_, AppState>) -> R<Vec<Playlist>> {
    state.db.run(|db| Ok(db.list_playlists()?)).await
}

// @req RF-06
#[tauri::command]
pub async fn playlist_create(
    state: State<'_, AppState>,
    name: String,
    description: Option<String>,
) -> R<Playlist> {
    let pl = state
        .db
        .run(move |db| Ok(db.create_playlist(&name, description.as_deref())?))
        .await?;
    state.events.library_changed("playlist_created");
    Ok(pl)
}

// @req RF-06
#[tauri::command]
pub async fn playlist_update(
    state: State<'_, AppState>,
    playlist_id: String,
    name: Option<String>,
    description: Option<String>,
) -> R<Playlist> {
    let patch = PlaylistPatch {
        name,
        description: description.map(Some),
        ..Default::default()
    };
    let pl = state
        .db
        .run(move |db| Ok(db.update_playlist(&playlist_id, &patch)?))
        .await?;
    state.events.library_changed("playlist_updated");
    Ok(pl)
}

// @req RF-06
#[tauri::command]
pub async fn playlist_delete(state: State<'_, AppState>, playlist_id: String) -> R<()> {
    state
        .db
        .run(move |db| Ok(db.delete_playlist(&playlist_id)?))
        .await?;
    state.events.library_changed("playlist_deleted");
    Ok(())
}

// @req RF-07 RF-08
#[tauri::command]
pub async fn playlist_get_tracks(
    state: State<'_, AppState>,
    playlist_id: String,
    query: Option<TrackQuery>,
) -> R<Vec<Track>> {
    let query = query.unwrap_or_default();
    state
        .db
        .run(move |db| Ok(db.playlist_tracks(&playlist_id, &query)?))
        .await
}

// @req RF-06
#[tauri::command]
pub async fn playlist_add_tracks(
    state: State<'_, AppState>,
    playlist_id: String,
    track_ids: Vec<String>,
) -> R<Playlist> {
    let pl = state
        .db
        .run(move |db| Ok(db.add_tracks_to_playlist(&playlist_id, &track_ids)?))
        .await?;
    state.events.library_changed("playlist_tracks_added");
    Ok(pl)
}

// @req RF-06
#[tauri::command]
pub async fn playlist_remove_tracks(
    state: State<'_, AppState>,
    playlist_id: String,
    track_ids: Vec<String>,
) -> R<Playlist> {
    let pl = state
        .db
        .run(move |db| Ok(db.remove_tracks_from_playlist(&playlist_id, &track_ids)?))
        .await?;
    state.events.library_changed("playlist_tracks_removed");
    Ok(pl)
}

// @req RF-07
#[tauri::command]
pub async fn playlist_move_track(
    state: State<'_, AppState>,
    playlist_id: String,
    track_id: String,
    new_position: i64,
) -> R<()> {
    state
        .db
        .run(move |db| Ok(db.move_playlist_track(&playlist_id, &track_id, new_position)?))
        .await?;
    state.events.library_changed("playlist_reordered");
    Ok(())
}
