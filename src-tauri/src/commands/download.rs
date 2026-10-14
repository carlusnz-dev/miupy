use miupy_core::DownloadJob;
use serde_json::json;
use tauri::State;

use crate::error::AppError;
use crate::state::AppState;

type R<T> = Result<T, AppError>;

/// Avisa o frontend dos jobs recém-enfileirados e acorda o trabalhador.
fn announce(state: &AppState, jobs: &[DownloadJob]) {
    for job in jobs {
        state.events.download_progress(job);
    }
    state.events.library_changed("download_queued");
    state.downloads.wake();
}

// @req RF-10
#[tauri::command]
pub async fn download_enqueue(
    state: State<'_, AppState>,
    track_ids: Vec<String>,
) -> R<Vec<DownloadJob>> {
    let jobs = state
        .db
        .run(move |db| {
            let jobs = db.enqueue_downloads(&track_ids)?;
            for j in &jobs {
                db.log_action("download_enqueued", Some("track"), Some(&j.track_id), None)?;
            }
            Ok(jobs)
        })
        .await?;
    announce(&state, &jobs);
    Ok(jobs)
}

// @req RF-11
#[tauri::command]
pub async fn download_enqueue_playlist(
    state: State<'_, AppState>,
    playlist_id: String,
) -> R<Vec<DownloadJob>> {
    let jobs = state
        .db
        .run(move |db| {
            let jobs = db.enqueue_playlist_downloads(&playlist_id)?;
            db.log_action(
                "playlist_download_enqueued",
                Some("playlist"),
                Some(&playlist_id),
                Some(&json!({ "tracks": jobs.len() })),
            )?;
            Ok(jobs)
        })
        .await?;
    announce(&state, &jobs);
    Ok(jobs)
}

// @req RF-10
#[tauri::command]
pub async fn download_list(state: State<'_, AppState>) -> R<Vec<DownloadJob>> {
    state.db.run(|db| Ok(db.list_downloads()?)).await
}

// @req RF-10
#[tauri::command]
pub async fn download_cancel(state: State<'_, AppState>, job_id: String) -> R<()> {
    let dl = state.downloads.clone();
    crate::db::blocking(move || dl.cancel(&job_id)).await
}
