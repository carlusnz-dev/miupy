//! Estado do app: tudo o que os comandos precisam, montado numa função testável.

use std::sync::Arc;

use miupy_audio::{AudioBackend, AudioError, AudioEvent, NullBackend, PlaybackState};
use miupy_core::Database;
use miupy_media::YtDlpManager;
use miupy_media::{MediaResolver, MetadataProvider};

use crate::db::Db;
use crate::downloads::Downloads;
use crate::error::AppResult;
use crate::events::Events;
use crate::metadata::MetadataService;
use crate::paths::Dirs;
use crate::player::Player;
use crate::recommendations::Recommendations;
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
    pub recommendations: Arc<Recommendations>,
    pub ytdlp: Arc<YtDlpManager>,
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

        let player = Player::new(
            db.clone(),
            backend,
            resolver.clone(),
            events.clone(),
            dirs.prefs_file.clone(),
        );
        player.restore_prefs();
        let metadata = Arc::new(MetadataService::new(
            db.clone(),
            provider.clone(),
            resolver.clone(),
            dirs.clone(),
            events.clone(),
        ));
        let downloads = Downloads::new(
            db.clone(),
            resolver.clone(),
            provider.clone(),
            metadata.clone(),
            events.clone(),
            dirs.clone(),
        );
        let youtube = Arc::new(Youtube::new(db.clone(), resolver.clone(), events.clone()));
        // @req RF-19
        let recommendations = Arc::new(Recommendations::new(
            db.clone(),
            resolver.clone(),
            events.clone(),
            youtube.clone(),
        ));
        let ytdlp = Arc::new(YtDlpManager::new(dirs.root.join("bin"))?);
        Ok(AppState {
            db,
            dirs,
            events,
            player,
            downloads,
            youtube,
            metadata,
            recommendations,
            ytdlp,
        })
    }

    /// Liga a thread de eventos do áudio e o trabalhador de downloads.
    pub fn start(&self) {
        self.player.spawn_event_loop();
        self.downloads.spawn_worker();
        // @req RF-28: varredura sequencial, fora da trava do player.
        #[cfg(not(target_os = "android"))]
        {
            let db = self.db.clone();
            std::thread::Builder::new()
                .name("miupy-loudness".into())
                .spawn(move || {
                    // @req RF-28: no Linux, o nice é por thread; a medição cede CPU ao player.
                    #[cfg(target_os = "linux")]
                    unsafe {
                        libc::setpriority(libc::PRIO_PROCESS, 0, 10);
                    }
                    let mut attempted = std::collections::HashSet::new();
                    loop {
                        if let Err(e) = measure_pending_loudness(&db, &mut attempted) {
                            log::warn!("fila de loudness: {e}");
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_secs(30));
                    }
                })
                .ok();
        }
        // @req RF-26: a checagem/instalação nunca bloqueia a abertura.
        let manager = self.ytdlp.clone();
        let events = self.events.clone();
        tauri::async_runtime::spawn(async move {
            let callback: miupy_media::YtDlpStatusFn = Arc::new(move |s| {
                let payload = crate::dto::from_ytdlp_status(s);
                events.ytdlp_status(&payload);
            });
            let _ = manager.ensure_current_with_status(Some(callback)).await;
        });
    }
}

// @req RF-28
#[cfg(not(target_os = "android"))]
fn measure_pending_loudness(
    db: &Db,
    attempted: &mut std::collections::HashSet<String>,
) -> miupy_core::Result<()> {
    let pending = db.with(|d| d.tracks_without_loudness())?;
    for (id, file) in pending {
        if attempted.contains(&id) || !std::path::Path::new(&file).is_file() {
            continue;
        }
        attempted.insert(id.clone());
        match miupy_audio::measure_file(std::path::Path::new(&file)) {
            Ok(Some(m)) => db.with(|d| d.set_loudness(&id, m.loudness_lufs, m.true_peak_db))?,
            Ok(None) => {}
            Err(e) => log::warn!("medir loudness de {id}: {e}"),
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    Ok(())
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
    fn set_normalization_gain_db(&self, gain_db: f32) -> miupy_audio::Result<()> {
        self.fallback.set_normalization_gain_db(gain_db)
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
    use crate::downloads::testing::{FakeMetadata, FakeResolver, sr};
    use crate::events::testing::RecordingEvents;
    use chrono::Datelike;
    use miupy_core::{DownloadStatus, NewTrack};

    // @test RF-03 RF-05 RF-10 RF-13 RF-14 RF-15 RF-18 RF-19 RF-20
    #[tokio::test]
    async fn fluxo_sem_janela_da_busca_ate_recomendacoes() {
        let tmp = tempfile::tempdir().unwrap();
        let backend = Arc::new(NullBackend::new());
        let resolver = Arc::new(FakeResolver::default());
        resolver.results.lock().unwrap().extend([
            sr("seed001", "Banda Pedra - Canção", "Banda Pedra"),
            sr("stream002", "Banda Pedra - Outra", "Banda Pedra"),
        ]);
        resolver.related.lock().unwrap().extend([
            sr("rec003", "Banda Pedra - Nova", "Banda Pedra"),
            sr("rec004", "Banda Pedra - Mais", "Banda Pedra"),
            miupy_media::SearchResult {
                duration_ms: None,
                ..sr("live005", "Ao vivo", "Canal")
            },
            miupy_media::SearchResult {
                duration_ms: Some(7_200_000),
                ..sr("mix006", "Mix longo", "Canal")
            },
        ]);
        let provider = Arc::new(FakeMetadata::default());
        *provider.recording.lock().unwrap() = Some(miupy_media::RecordingMatch {
            mbid: "mb-1".into(),
            title: "Canção".into(),
            artist: "Banda Pedra".into(),
            artist_mbid: None,
            album: Some("Disco".into()),
            release_mbid: None,
            release_mbids: vec![],
            duration_ms: Some(200_000),
            score: 100,
            genres: vec!["rock".into()],
            release_year: Some(2024),
            isrc: Some("BRABC2400001".into()),
        });
        let events = Arc::new(RecordingEvents::default());
        let st = AppState::new(Parts {
            dirs: Dirs::new(tmp.path()),
            events: events.clone(),
            backend: backend.clone(),
            resolver: resolver.clone(),
            provider,
        })
        .unwrap();
        st.youtube.search("Banda Pedra", None).await.unwrap();
        let tracks = st
            .youtube
            .add_to_library(vec!["seed001".into(), "stream002".into()], None)
            .await
            .unwrap();
        st.db
            .with(|db| db.enqueue_downloads(&[tracks[0].id.clone()]))
            .unwrap();
        st.downloads.run_until_idle().await;
        let first = st.db.with(|db| db.get_track(&tracks[0].id)).unwrap();
        assert!(first.file_path.is_some());
        assert_eq!(first.genres, vec!["rock"]);
        assert_eq!(first.release_year, Some(2024));
        assert_eq!(
            st.db
                .with(|db| db.list_tracks_by_artist(
                    first.artist_id.as_deref().unwrap(),
                    &miupy_core::TrackQuery::default()
                ))
                .unwrap()
                .len(),
            2
        );
        assert_eq!(st.db.with(|db| db.clear_finished_downloads()).unwrap(), 1);

        let queue_ids = tracks.iter().map(|t| t.id.clone()).collect::<Vec<_>>();
        st.player.play(&first.id, Some(queue_ids.clone())).unwrap();
        let local_event_id = st.player.current_event_id().unwrap();
        assert_eq!(
            st.player
                .queue_tracks()
                .unwrap()
                .iter()
                .map(|t| &t.id)
                .collect::<Vec<_>>(),
            queue_ids.iter().collect::<Vec<_>>()
        );
        for _ in 0..360 {
            backend.advance(500);
            st.player.process_pending_events();
        }
        let local_event = st.db.with(|db| db.get_play_event(&local_event_id)).unwrap();
        assert!(local_event.completed);
        assert!(local_event.listened_ms > 0);
        assert!(st.player.state().streaming);
        let stream_event_id = st.player.current_event_id().unwrap();
        assert_eq!(st.player.state().track.as_ref().unwrap().id, tracks[1].id);
        assert_eq!(st.db.with(|db| db.library_stats()).unwrap().track_count, 2);
        let year = chrono::Local::now().year();
        let tz = crate::tz::local_offset_minutes();
        assert!(
            st.db
                .with(|db| db.wrapped_years(tz))
                .unwrap()
                .contains(&year)
        );
        let wrapped = st.db.with(|db| db.wrapped(year, tz)).unwrap();
        assert!(wrapped.total_listened_ms > 0);
        st.player.play(&first.id, None).unwrap(); // pula a faixa em streaming
        assert!(
            st.db
                .with(|db| db.get_play_event(&stream_event_id))
                .unwrap()
                .skipped
        );
        st.player.stop().unwrap();
        let taste = st.db.with(|db| db.taste_profile(None)).unwrap();
        assert!(taste.total_listened_ms > 0);

        let found = st.recommendations.refresh(None).await.unwrap();
        assert_eq!(found.len(), 2);
        assert!(!found[0].reasons.is_empty());
        assert_eq!(
            found.iter().map(|r| &r.source_id).collect::<Vec<_>>(),
            st.recommendations
                .list(None, None)
                .await
                .unwrap()
                .iter()
                .map(|r| &r.source_id)
                .collect::<Vec<_>>()
        );
        let playlist = st.db.with(|db| db.create_playlist("Pedras", None)).unwrap();
        st.db
            .with(|db| db.add_tracks_to_playlist(&playlist.id, std::slice::from_ref(&first.id)))
            .unwrap();
        assert_eq!(
            st.recommendations
                .refresh(Some(playlist.id.clone()))
                .await
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            st.recommendations
                .list(Some(playlist.id), Some(1))
                .await
                .unwrap()
                .len(),
            1
        );
        *resolver.always_fail_network.lock().unwrap() = true;
        assert_eq!(
            st.recommendations.refresh(None).await.unwrap_err().code(),
            "network"
        );
        assert_eq!(st.recommendations.list(None, None).await.unwrap().len(), 2);
        *resolver.always_fail_network.lock().unwrap() = false;
        let added = st
            .youtube
            .add_to_library(vec![found[0].source_id.clone()], None)
            .await
            .unwrap();
        assert_eq!(added.len(), 1);
        assert!(
            !st.recommendations
                .list(None, None)
                .await
                .unwrap()
                .iter()
                .any(|r| r.source_id == found[0].source_id)
        );
        let other = found[1].source_id.clone();
        st.recommendations.dismiss(other.clone()).await.unwrap();
        st.recommendations.refresh(None).await.unwrap();
        assert!(
            !st.recommendations
                .list(None, None)
                .await
                .unwrap()
                .iter()
                .any(|r| r.source_id == other)
        );
        assert!(!events.recommendations.lock().unwrap().is_empty());
    }

    // @test RF-18
    #[tokio::test]
    async fn streaming_sem_rede_devolve_network_e_nao_abre_evento() {
        let tmp = tempfile::tempdir().unwrap();
        let resolver = Arc::new(FakeResolver::default());
        let events = Arc::new(RecordingEvents::default());
        *resolver.stream_fail.lock().unwrap() =
            Some(miupy_media::MediaError::Network("offline".into()));
        let st = AppState::new(Parts {
            dirs: Dirs::new(tmp.path()),
            events: events.clone(),
            backend: Arc::new(NullBackend::new()),
            resolver,
            provider: Arc::new(FakeMetadata::default()),
        })
        .unwrap();
        let track = st
            .db
            .with(|db| {
                db.upsert_track(&NewTrack::youtube(
                    "offline01",
                    "Sem rede",
                    None,
                    200_000,
                    None,
                ))
            })
            .unwrap()
            .track;
        let initial = st.player.play(&track.id, None).unwrap();
        assert_eq!(initial.status, crate::dto::PlayerStatus::Loading);
        for _ in 0..100 {
            if st.player.state().status == crate::dto::PlayerStatus::Stopped {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
        assert_eq!(st.player.state().status, crate::dto::PlayerStatus::Stopped);
        assert_eq!(events.player_errors.lock().unwrap().len(), 1);
    }

    // @test RF-18 RF-13
    #[tokio::test]
    async fn streaming_renova_url_expirada_e_recupera_um_erro() {
        let tmp = tempfile::tempdir().unwrap();
        let resolver = Arc::new(FakeResolver::default());
        *resolver.stream_expiry.lock().unwrap() = Some(0);
        let backend = Arc::new(NullBackend::new());
        let st = AppState::new(Parts {
            dirs: Dirs::new(tmp.path()),
            events: Arc::new(RecordingEvents::default()),
            backend,
            resolver: resolver.clone(),
            provider: Arc::new(FakeMetadata::default()),
        })
        .unwrap();
        let track = st
            .db
            .with(|db| {
                db.upsert_track(&NewTrack::youtube(
                    "renova001",
                    "Faixa",
                    None,
                    200_000,
                    None,
                ))
            })
            .unwrap()
            .track;
        st.player.play(&track.id, None).unwrap();
        for _ in 0..100 {
            if st.player.state().status == crate::dto::PlayerStatus::Playing {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
        st.player.pause().unwrap();
        st.player.resume().unwrap();
        assert_eq!(*resolver.stream_calls.lock().unwrap(), 2);
        st.player.handle_event(AudioEvent::Error {
            message: "rede caiu".into(),
        });
        assert!(st.player.state().streaming);
        assert_eq!(*resolver.stream_calls.lock().unwrap(), 3);
        st.player.handle_event(AudioEvent::Error {
            message: "rede caiu de novo".into(),
        });
        assert_eq!(st.player.state().status, crate::dto::PlayerStatus::Stopped);
        assert!(st.player.current_event_id().is_none());
    }

    // @test RF-18
    #[tokio::test]
    async fn streaming_lento_nao_trava_comandos_e_a_resolucao_antiga_e_descartada() {
        let tmp = tempfile::tempdir().unwrap();
        let resolver = Arc::new(FakeResolver::default());
        *resolver.stream_delay_ms.lock().unwrap() = 350;
        let st = AppState::new(Parts {
            dirs: Dirs::new(tmp.path()),
            events: Arc::new(RecordingEvents::default()),
            backend: Arc::new(NullBackend::new()),
            resolver: resolver.clone(),
            provider: Arc::new(FakeMetadata::default()),
        })
        .unwrap();
        let tracks = st
            .db
            .with(|db| {
                Ok::<_, miupy_core::Error>(vec![
                    db.upsert_track(&NewTrack::youtube("slow001", "slow", None, 200_000, None))?
                        .track,
                    db.upsert_track(&NewTrack::youtube("fast002", "fast", None, 200_000, None))?
                        .track,
                ])
            })
            .unwrap();
        let start = std::time::Instant::now();
        let first = st.player.play(&tracks[0].id, None).unwrap();
        assert!(start.elapsed() < std::time::Duration::from_millis(100));
        assert_eq!(first.status, crate::dto::PlayerStatus::Loading);
        st.player.pause().unwrap();
        let s = st.player.state();
        assert_eq!(s.track.as_ref().unwrap().id, tracks[0].id);
        st.player.play(&tracks[1].id, None).unwrap();
        for _ in 0..200 {
            if st.player.state().status == crate::dto::PlayerStatus::Playing {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
        let s = st.player.state();
        assert_eq!(s.track.as_ref().unwrap().id, tracks[1].id);
        assert_eq!(s.status, crate::dto::PlayerStatus::Playing);
    }

    // @test RF-18 RF-23 RF-26 RF-27
    #[tokio::test]
    #[ignore = "rede real: baixa yt-dlp e consulta YouTube, MusicBrainz e Wikimedia"]
    async fn integracao_real_ytdlp_stream_sugestoes_e_foto_de_artista() {
        use miupy_media::{MetadataConfig, OnlineMetadata, YtDlpManager, YtDlpResolver};
        let tmp = tempfile::tempdir().unwrap();
        let manager = YtDlpManager::new(tmp.path().join("bin")).unwrap();
        let status = manager.update(true, None).await.unwrap();
        assert_eq!(status.state, miupy_media::YtDlpState::Ready);
        assert!(status.path.as_ref().unwrap().is_file());
        let suggestions = miupy_media::suggest("radiohead creep").await.unwrap();
        assert!(!suggestions.is_empty());
        let provider = Arc::new(OnlineMetadata::new(MetadataConfig::new("miupy test")).unwrap());
        let artist_mbid = "056e4f3e-d505-4dad-8ec1-d04f521cbb56";
        // Primeiro direto no provedor, para a falha dizer de onde veio (rede, MBID, imagem).
        let direct = provider.artist_photo(Some(artist_mbid)).await;
        assert!(
            matches!(direct, Ok(Some(_))),
            "foto direta do provedor: {:?}",
            direct
                .as_ref()
                .map(|o| o.as_ref().map(|p| p.credit.clone()))
        );
        let resolver = Arc::new(YtDlpResolver::with_binary(status.path.unwrap()));
        let st = AppState::new(Parts {
            dirs: Dirs::new(tmp.path().join("app")),
            events: Arc::new(RecordingEvents::default()),
            backend: Arc::new(NullBackend::new()),
            resolver,
            provider,
        })
        .unwrap();
        let track = st
            .db
            .with(|db| {
                Ok::<_, miupy_core::Error>(
                    db.upsert_track(&NewTrack::youtube(
                        "dQw4w9WgXcQ",
                        "Never Gonna Give You Up",
                        Some("Rick Astley".into()),
                        213_000,
                        None,
                    ))?
                    .track,
                )
            })
            .unwrap();
        let artist_id = track.artist_id.clone().unwrap();
        st.db
            .with(|db| {
                db.update_artist(
                    &artist_id,
                    &miupy_core::ArtistPatch {
                        mbid: Some(Some(artist_mbid.into())),
                        ..Default::default()
                    },
                )
            })
            .unwrap();
        let artist = st.metadata.get_artist(&artist_id).await.unwrap();
        let photo_path = artist.image_path.expect("foto real gravada");
        assert!(std::path::Path::new(&photo_path).is_file());
        assert!(
            artist
                .image_credit
                .as_deref()
                .is_some_and(|credit| !credit.is_empty())
        );
        let before = std::time::Instant::now();
        let state = st.player.play(&track.id, None).unwrap();
        assert!(before.elapsed() < std::time::Duration::from_millis(100));
        assert_eq!(state.status, crate::dto::PlayerStatus::Loading);
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(40);
        while st.player.state().status != crate::dto::PlayerStatus::Playing
            && tokio::time::Instant::now() < deadline
        {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        assert_eq!(st.player.state().status, crate::dto::PlayerStatus::Playing);
    }

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

#[cfg(all(test, not(target_os = "android")))]
mod loudness_tests {
    use super::*;
    use miupy_core::{DownloadedFile, NewTrack};
    // @test RF-28
    #[test]
    fn old_download_is_measured_once_without_player() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("old.m4a");
        std::fs::write(
            &path,
            include_bytes!("../../crates/miupy-audio/tests/fixtures/tone.m4a"),
        )
        .unwrap();
        let db = Db::new(Database::open_in_memory().unwrap());
        let track = db
            .with(|d| d.upsert_track(&NewTrack::youtube("old", "Antiga", None, 1000, None)))
            .unwrap()
            .track;
        db.with(|d| {
            d.mark_downloaded(
                &track.id,
                &DownloadedFile {
                    file_path: path.to_string_lossy().into_owned(),
                    codec: None,
                    bitrate: None,
                    cover_path: None,
                    duration_ms: None,
                },
            )
        })
        .unwrap();
        let mut attempted = std::collections::HashSet::new();
        measure_pending_loudness(&db, &mut attempted).unwrap();
        assert!(
            db.with(|d| d.get_track(&track.id))
                .unwrap()
                .loudness_lufs
                .is_some()
        );
        assert_eq!(attempted.len(), 1);
        measure_pending_loudness(&db, &mut attempted).unwrap();
        assert_eq!(attempted.len(), 1);
    }
}
