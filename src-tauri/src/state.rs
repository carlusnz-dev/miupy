//! Estado do app: tudo o que os comandos precisam, montado numa função testável.

use std::sync::Arc;

use miupy_audio::{AudioBackend, AudioError, AudioEvent, NullBackend, PlaybackState};
use miupy_core::Database;
use miupy_media::{MediaResolver, MetadataProvider};

use crate::db::Db;
use crate::downloads::Downloads;
use crate::error::AppResult;
use crate::events::Events;
use crate::metadata::MetadataService;
use crate::paths::Dirs;
use crate::player::Player;
use crate::youtube::Youtube;

/// Estado compartilhado, registrado no Tauri com `app.manage`.
#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub dirs: Dirs,
    pub events: Arc<dyn Events>,
    pub player: Arc<Player>,
    pub downloads: Arc<Downloads>,
    pub youtube: Arc<Youtube>,
    pub metadata: Arc<MetadataService>,
}

/// Dependências externas injetadas (trocadas por dublês nos testes).
pub struct Parts {
    pub dirs: Dirs,
    pub events: Arc<dyn Events>,
    pub backend: Arc<dyn AudioBackend>,
    pub resolver: Arc<dyn MediaResolver>,
    pub provider: Arc<dyn MetadataProvider>,
}

impl AppState {
    /// Cria as pastas, abre o banco (migrando), faz a limpeza de boot e monta os serviços.
    /// Não inicia threads nem tarefas: veja [`AppState::start`].
    pub fn new(parts: Parts) -> AppResult<AppState> {
        let Parts {
            dirs,
            events,
            backend,
            resolver,
            provider,
        } = parts;
        dirs.create_all()?;
        let db = Db::new(Database::open(&dirs.db_file)?);

        // Limpeza de boot: downloads que estavam em curso voltam à fila e reproduções que ficaram
        // abertas por queda do processo são fechadas.
        let requeued = db.with(|db| db.requeue_interrupted_downloads())?;
        let closed = db.with(|db| db.close_dangling_play_events())?;
        if requeued > 0 || closed > 0 {
            log::info!(
                "boot: {requeued} download(s) recolocados na fila, {closed} reprodução(ões) fechadas"
            );
        }

        let player = Player::new(db.clone(), backend, events.clone(), dirs.prefs_file.clone());
        player.restore_prefs();
        let downloads = Downloads::new(
            db.clone(),
            resolver.clone(),
            provider.clone(),
            events.clone(),
            dirs.clone(),
        );
        let youtube = Arc::new(Youtube::new(db.clone(), resolver, events.clone()));
        let metadata = Arc::new(MetadataService::new(
            db.clone(),
            provider,
            dirs.clone(),
            events.clone(),
        ));
        Ok(AppState {
            db,
            dirs,
            events,
            player,
            downloads,
            youtube,
            metadata,
        })
    }

    /// Liga a thread de eventos do áudio e o trabalhador de downloads.
    pub fn start(&self) {
        self.player.spawn_event_loop();
        self.downloads.spawn_worker();
    }
}

/// Backend usado quando o motor de áudio não abre (sem dispositivo, por exemplo): o app abre
/// e navega normalmente; tocar devolve o erro explicando o motivo.
pub struct UnavailableBackend {
    reason: String,
    fallback: NullBackend,
}

impl UnavailableBackend {
    pub fn new(reason: impl Into<String>) -> Self {
        UnavailableBackend {
            reason: reason.into(),
            fallback: NullBackend::new(),
        }
    }

    fn err(&self) -> AudioError {
        AudioError::NoOutputDevice(self.reason.clone())
    }
}

impl AudioBackend for UnavailableBackend {
    fn load(&self, _path: &std::path::Path) -> miupy_audio::Result<()> {
        Err(self.err())
    }
    fn load_url(
        &self,
        _url: &str,
        _options: miupy_audio::StreamOptions,
    ) -> miupy_audio::Result<()> {
        Err(self.err())
    }
    fn play(&self) -> miupy_audio::Result<()> {
        Err(self.err())
    }
    fn pause(&self) -> miupy_audio::Result<()> {
        Ok(())
    }
    fn resume(&self) -> miupy_audio::Result<()> {
        Err(self.err())
    }
    fn stop(&self) -> miupy_audio::Result<()> {
        Ok(())
    }
    fn seek(&self, _position_ms: u64) -> miupy_audio::Result<()> {
        Err(self.err())
    }
    fn set_volume(&self, volume: f32) -> miupy_audio::Result<()> {
        self.fallback.set_volume(volume)
    }
    fn set_muted(&self, muted: bool) -> miupy_audio::Result<()> {
        self.fallback.set_muted(muted)
    }
    fn state(&self) -> PlaybackState {
        self.fallback.state()
    }
    fn events(&self) -> crossbeam_channel::Receiver<AudioEvent> {
        self.fallback.events()
    }
}

/// Backend de áudio do sistema; se não abrir, devolve o [`UnavailableBackend`] e registra o motivo.
pub fn default_backend() -> Arc<dyn AudioBackend> {
    match miupy_audio::create_default_backend() {
        Ok(b) => Arc::from(b),
        Err(e) => {
            log::error!("motor de áudio indisponível: {e}");
            Arc::new(UnavailableBackend::new(e.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::downloads::testing::{FakeMetadata, FakeResolver};
    use crate::events::testing::RecordingEvents;
    use miupy_core::{DownloadStatus, NewTrack};

    fn parts(root: &std::path::Path, backend: Arc<dyn AudioBackend>) -> Parts {
        Parts {
            dirs: Dirs::new(root),
            events: Arc::new(RecordingEvents::default()),
            backend,
            resolver: Arc::new(FakeResolver::default()),
            provider: Arc::new(FakeMetadata::default()),
        }
    }

    // @test RNF-01 RF-10 RF-13
    #[test]
    fn boot_cria_pastas_e_faz_a_limpeza() {
        let tmp = tempfile::tempdir().unwrap();
        // primeira abertura: cria banco, pastas e dados "deixados" por uma execução anterior
        let st = AppState::new(parts(tmp.path(), Arc::new(NullBackend::new()))).unwrap();
        assert!(tmp.path().join("miupy.db").is_file());
        assert!(tmp.path().join("audio").is_dir());
        assert!(tmp.path().join("covers").is_dir());
        let t = st
            .db
            .with(|db| db.upsert_track(&NewTrack::youtube("v1", "T", None, 1000, None)))
            .unwrap()
            .track;
        st.db
            .with(|db| db.enqueue_downloads(std::slice::from_ref(&t.id)))
            .unwrap();
        let job = st.db.with(|db| db.next_queued_download()).unwrap().unwrap();
        st.db
            .with(|db| {
                db.update_download(
                    &job.id,
                    &miupy_core::DownloadUpdate {
                        status: DownloadStatus::Downloading,
                        progress: Some(0.5),
                        error: None,
                    },
                )
            })
            .unwrap();
        let ev = st.db.with(|db| db.start_play(&t.id, None)).unwrap();
        drop(st);

        // segunda abertura (o "boot"): o download volta à fila e a reprodução é fechada
        let st = AppState::new(parts(tmp.path(), Arc::new(NullBackend::new()))).unwrap();
        let job = st.db.with(|db| db.get_download(&job.id)).unwrap();
        assert_eq!(job.status, DownloadStatus::Queued);
        let ev = st.db.with(|db| db.get_play_event(&ev)).unwrap();
        assert!(ev.ended_at.is_some());
    }

    // @test RF-01
    #[test]
    fn sem_dispositivo_o_app_abre_e_tocar_explica_o_motivo() {
        let tmp = tempfile::tempdir().unwrap();
        let st = AppState::new(parts(
            tmp.path(),
            Arc::new(UnavailableBackend::new("sem placa de som")),
        ))
        .unwrap();
        let file = tmp.path().join("audio/x.m4a");
        std::fs::write(&file, b"x").unwrap();
        let t = st
            .db
            .with(|db| db.upsert_track(&NewTrack::youtube("v1", "T", None, 1000, None)))
            .unwrap()
            .track;
        st.db
            .with(|db| {
                db.mark_downloaded(
                    &t.id,
                    &miupy_core::DownloadedFile {
                        file_path: file.to_string_lossy().into_owned(),
                        codec: None,
                        bitrate: None,
                        cover_path: None,
                        duration_ms: None,
                    },
                )
            })
            .unwrap();
        let err = st.player.play(&t.id, None).unwrap_err();
        assert_eq!(err.code(), "unavailable");
        assert!(err.to_string().contains("sem placa de som"));
        assert_eq!(st.player.state().status, crate::dto::PlayerStatus::Stopped);
    }
}
