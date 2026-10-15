//! Liga playlists à fila única de downloads.

use miupy_core::DownloadStatus;

use crate::error::AppResult;
use crate::state::AppState;

// @req RF-31
/// Enfileira apenas faixas elegíveis e emite um resumo por operação.
pub async fn enqueue(
    state: &AppState,
    playlist_id: String,
    track_ids: Option<Vec<String>>,
    reason: &'static str,
) -> AppResult<usize> {
    let id = playlist_id.clone();
    let (jobs, enabled) = state
        .db
        .run(move |db| {
            if !db.get_playlist(&id)?.auto_download {
                return Ok((Vec::new(), false));
            }
            let ids = match track_ids {
                Some(ids) => {
                    let mut eligible = Vec::new();
                    for track_id in ids {
                        let status = db.get_track(&track_id)?.download_status;
                        if matches!(status, DownloadStatus::None | DownloadStatus::Error)
                            && db.auto_playlists_for_track(&track_id)?.contains(&id)
                        {
                            eligible.push(track_id);
                        }
                    }
                    eligible
                }
                None => db.playlist_auto_pending_tracks(&id)?,
            };
            Ok((db.enqueue_downloads(&ids)?, true))
        })
        .await?;
    if !jobs.is_empty() {
        for job in &jobs {
            state.events.download_progress(job);
        }
        state.events.library_changed("download_queued");
        state.downloads.wake();
    }
    if enabled {
        state.events.download_auto(&playlist_id, jobs.len(), reason);
    }
    Ok(jobs.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::downloads::testing::{FakeMetadata, FakeResolver};
    use crate::events::testing::RecordingEvents;
    use crate::paths::Dirs;
    use crate::state::{AppState, Parts};
    use miupy_core::{NewTrack, PlaylistPatch};
    use std::sync::Arc;

    // @test RF-31
    #[tokio::test]
    async fn ligar_enfileira_so_pendentes_desligar_preserva_e_adicao_emite_uma_vez() {
        let tmp = tempfile::tempdir().unwrap();
        let events = Arc::new(RecordingEvents::default());
        let st = AppState::new(Parts {
            dirs: Dirs::new(tmp.path()),
            events: events.clone(),
            backend: Arc::new(miupy_audio::NullBackend::new()),
            resolver: Arc::new(FakeResolver::default()),
            provider: Arc::new(FakeMetadata::default()),
        })
        .unwrap();
        let (p, a, b, local) = st
            .db
            .with(|db| {
                let p = db.create_playlist("Auto", None)?;
                let a = db
                    .upsert_track(&NewTrack::youtube("aa", "A", None, 1000, None))?
                    .track;
                let b = db
                    .upsert_track(&NewTrack::youtube("bb", "B", None, 1000, None))?
                    .track;
                let local = db
                    .upsert_track(&NewTrack::local("/tmp/auto.mp3", "Local"))?
                    .track;
                db.add_tracks_to_playlist(&p.id, &[a.id.clone(), local.id.clone()])?;
                db.update_playlist(
                    &p.id,
                    &PlaylistPatch {
                        auto_download: Some(true),
                        ..Default::default()
                    },
                )?;
                Ok::<_, miupy_core::Error>((p, a, b, local))
            })
            .unwrap();
        assert_eq!(
            enqueue(&st, p.id.clone(), None, "enabled").await.unwrap(),
            1
        );
        assert_eq!(
            events.auto.lock().unwrap().as_slice(),
            &[(p.id.clone(), 1, "enabled".into())]
        );
        assert_eq!(st.db.with(|db| db.list_downloads()).unwrap().len(), 1);
        st.db
            .with(|db| {
                db.update_playlist(
                    &p.id,
                    &PlaylistPatch {
                        auto_download: Some(false),
                        ..Default::default()
                    },
                )
            })
            .unwrap();
        assert_eq!(
            enqueue(&st, p.id.clone(), Some(vec![b.id.clone()]), "added")
                .await
                .unwrap(),
            0
        );
        assert_eq!(st.db.with(|db| db.list_downloads()).unwrap().len(), 1);
        st.db
            .with(|db| {
                db.update_playlist(
                    &p.id,
                    &PlaylistPatch {
                        auto_download: Some(true),
                        ..Default::default()
                    },
                )?;
                db.add_tracks_to_playlist(&p.id, std::slice::from_ref(&b.id))?;
                Ok::<(), miupy_core::Error>(())
            })
            .unwrap();
        assert_eq!(
            enqueue(
                &st,
                p.id.clone(),
                Some(vec![b.id.clone(), local.id.clone()]),
                "added"
            )
            .await
            .unwrap(),
            1
        );
        assert_eq!(
            events.auto.lock().unwrap().last().unwrap(),
            &(p.id.clone(), 1, "added".into())
        );
        assert_eq!(st.db.with(|db| db.list_downloads()).unwrap().len(), 2);
        assert_eq!(
            st.db
                .with(|db| db.get_track(&a.id))
                .unwrap()
                .download_status,
            DownloadStatus::Queued
        );
    }
}
