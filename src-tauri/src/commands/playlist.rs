use miupy_core::{Playlist, PlaylistPatch, Track, TrackQuery};
use tauri::State;

use crate::dto::ImageUpload;
use crate::error::AppError;
use crate::state::AppState;

type R<T> = Result<T, AppError>;

// @req RF-22
#[tauri::command]
pub async fn playlist_set_cover(
    state: State<'_, AppState>,
    playlist_id: String,
    image: Option<ImageUpload>,
) -> R<Playlist> {
    let new_path = image
        .as_ref()
        .map(|i| crate::paths::save_upload(i, &state.dirs.root.join("playlists"), "playlist"))
        .transpose()?;
    let path = new_path.as_ref().map(|p| p.to_string_lossy().into_owned());
    let update_id = playlist_id.clone();
    let old = state
        .db
        .run(move |db| Ok(db.set_playlist_cover(&update_id, path.as_deref())?))
        .await?;
    if let Some(old) = old {
        let p = std::path::PathBuf::from(old);
        if state.dirs.owns(&p) {
            let _ = std::fs::remove_file(p);
        }
    }
    state.events.library_changed("playlist_cover_updated");
    let id = playlist_id;
    state
        .db
        .run(move |db| {
            Ok(db
                .list_playlists()?
                .into_iter()
                .find(|p| p.id == id)
                .ok_or_else(|| miupy_core::Error::NotFound("playlist".into()))?)
        })
        .await
}

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
    let was_favorites = state
        .db
        .run(move |db| {
            let was_favorites = db.get_playlist(&playlist_id)?.is_favorites;
            db.delete_playlist(&playlist_id)?;
            Ok(was_favorites)
        })
        .await?;
    state.events.library_changed("playlist_deleted");
    if was_favorites {
        state.events.library_changed("favorites");
    }
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
    if pl.is_favorites {
        state.events.library_changed("favorites");
    }
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
    if pl.is_favorites {
        state.events.library_changed("favorites");
    }
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

// @req RF-30
#[tauri::command]
pub async fn favorites_set_playlist(
    state: State<'_, AppState>,
    playlist_id: String,
) -> R<Playlist> {
    let (playlist, changed) = state
        .db
        .run(move |db| {
            let changed = db.get_settings()?.favorites_playlist_id.as_deref() != Some(&playlist_id);
            Ok((db.favorites_set_playlist(&playlist_id)?, changed))
        })
        .await?;
    if changed {
        state.events.library_changed("favorites");
    }
    Ok(playlist)
}
