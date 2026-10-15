use miupy_core::Track;
use tauri::State;

use crate::dto::SearchResult;
use crate::dto::YtDlpStatus;
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

// @req RF-09 RF-31
#[tauri::command]
pub async fn youtube_add_to_library(
    state: State<'_, AppState>,
    source_ids: Vec<String>,
    playlist_id: Option<String>,
) -> R<Vec<Track>> {
    let before = if let Some(id) = &playlist_id {
        let id = id.clone();
        state
            .db
            .run(move |db| {
                Ok(db
                    .playlist_tracks(&id, &miupy_core::TrackQuery::default())?
                    .into_iter()
                    .map(|t| t.id)
                    .collect::<std::collections::HashSet<_>>())
            })
            .await?
    } else {
        Default::default()
    };
    let tracks = state
        .youtube
        .add_to_library(source_ids, playlist_id.clone())
        .await?;
    if let Some(id) = playlist_id {
        let added = tracks
            .iter()
            .filter(|t| !before.contains(&t.id))
            .map(|t| t.id.clone())
            .collect();
        crate::auto_download::enqueue(&state, id, Some(added), "added").await?;
    }
    Ok(tracks)
}

// @req RF-23
#[tauri::command]
pub async fn youtube_suggest(query: String) -> R<Vec<String>> {
    Ok(miupy_media::suggest(&query).await?)
}

// @req RF-26
#[tauri::command]
pub async fn ytdlp_status(state: State<'_, AppState>) -> R<YtDlpStatus> {
    Ok(state.ytdlp.status().await.into())
}

// @req RF-26
#[tauri::command]
pub async fn ytdlp_update(state: State<'_, AppState>) -> R<YtDlpStatus> {
    let events = state.events.clone();
    let callback: miupy_media::YtDlpStatusFn =
        std::sync::Arc::new(move |s| events.ytdlp_status(&s.into()));
    Ok(state
        .ytdlp
        .update_with_status(true, Some(callback))
        .await?
        .into())
}
