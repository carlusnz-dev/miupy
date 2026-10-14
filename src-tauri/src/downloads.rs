//! Gerenciador de downloads (RF-10, RF-11): trabalhador em segundo plano que consome a fila do
//! banco (`next_queued_download`) com concorrência limitada.
//!
//! Para cada job: baixa o áudio com o `MediaResolver` (destino `audio/<id da faixa>.m4a`),
//! grava o progresso no banco (com limite de frequência) e o emite em `download://progress`;
//! ao concluir baixa a capa, grava as tags e só então marca a faixa como baixada.
//! Cancelar apaga o job no banco; o downloader percebe pelo `not_found` do `update_download`
//! ou pelo `CancellationToken`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use miupy_core::{
    DownloadJob, DownloadStatus, DownloadUpdate, DownloadedFile, Source, Track, TrackPatch,
};
use miupy_media::{DownloadRequest, MediaResolver, MetadataProvider, TagUpdate, tagger};
use tokio::sync::{Notify, Semaphore, watch};
use tokio_util::sync::CancellationToken;

use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::events::Events;
use crate::paths::Dirs;

/// Downloads simultâneos.
pub const MAX_CONCURRENT: usize = 2;
/// Intervalo mínimo entre gravações de progresso no banco.
const PROGRESS_INTERVAL: Duration = Duration::from_millis(400);

pub struct Downloads {
    db: Db,
    resolver: Arc<dyn MediaResolver>,
    metadata: Arc<dyn MetadataProvider>,
    events: Arc<dyn Events>,
    dirs: Dirs,
    max_concurrent: usize,
    wake: Notify,
    active: Mutex<HashMap<String, CancellationToken>>,
}

impl Downloads {
    pub fn new(
        db: Db,
        resolver: Arc<dyn MediaResolver>,
        metadata: Arc<dyn MetadataProvider>,
        events: Arc<dyn Events>,
        dirs: Dirs,
    ) -> Arc<Self> {
        Arc::new(Downloads {
            db,
            resolver,
            metadata,
            events,
            dirs,
            max_concurrent: MAX_CONCURRENT,
            wake: Notify::new(),
            active: Mutex::new(HashMap::new()),
        })
    }

    /// Avisa o trabalhador de que há jobs novos (guarda o aviso se ele estiver ocupado).
    pub fn wake(&self) {
        self.wake.notify_one();
    }

    // @req RF-10
    /// Inicia o trabalhador no runtime do Tauri.
    pub fn spawn_worker(self: &Arc<Self>) {
        let this = Arc::clone(self);
        tauri::async_runtime::spawn(async move { this.run().await });
        self.wake();
    }

    // @req RF-10
    /// Laço do trabalhador: pega uma vaga, reivindica o próximo job `queued` e o processa.
    pub async fn run(self: Arc<Self>) {
        let slots = Arc::new(Semaphore::new(self.max_concurrent));
        loop {
            let Ok(permit) = slots.clone().acquire_owned().await else {
                return;
            };
            let (job, token) = loop {
                match self.claim_next().await {
                    Ok(Some(claimed)) => break claimed,
                    Ok(None) => self.wake.notified().await,
                    Err(e) => {
                        log::error!("falha ao ler a fila de downloads: {e}");
                        tokio::time::sleep(Duration::from_secs(2)).await;
                    }
                }
            };
            let this = Arc::clone(&self);
            tauri::async_runtime::spawn(async move {
                this.process(job, token).await;
                drop(permit);
                this.wake(); // com a vaga livre, pode haver mais jobs esperando
            });
        }
    }

    /// Processa a fila inteira em sequência e volta (usado nos testes).
    #[cfg(test)]
    pub async fn run_until_idle(&self) {
        while let Some((job, token)) = self.claim_next().await.unwrap() {
            self.process(job, token).await;
        }
    }

    /// Pega o job `queued` mais antigo e o marca `downloading` na mesma operação.
    async fn claim_next(&self) -> AppResult<Option<(DownloadJob, CancellationToken)>> {
        let job = self
            .db
            .run(|db| {
                let Some(next) = db.next_queued_download()? else {
                    return Ok(None);
                };
                let claimed = db.update_download(
                    &next.id,
                    &DownloadUpdate {
                        status: DownloadStatus::Downloading,
                        progress: Some(0.0),
                        error: None,
                    },
                )?;
                Ok(Some(claimed))
            })
            .await?;
        Ok(job.map(|job| {
            let token = CancellationToken::new();
            self.active_lock().insert(job.id.clone(), token.clone());
            (job, token)
        }))
    }

    fn active_lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, CancellationToken>> {
        self.active.lock().unwrap_or_else(|e| e.into_inner())
    }

    // @req RF-10
    /// Cancela um job: apaga o registro, interrompe o download em curso e avisa o frontend.
    pub fn cancel(&self, job_id: &str) -> AppResult<()> {
        let job = self.db.with(|db| {
            let job = db.get_download(job_id)?;
            db.cancel_download(job_id)?;
            AppResult::Ok(job)
        })?;
        if let Some(token) = self.active_lock().get(job_id) {
            token.cancel();
        }
        // O job deixa de existir; o evento final leva `none` para a lista do frontend o tirar.
        self.events.download_progress(&DownloadJob {
            id: job.id,
            track_id: job.track_id,
            status: DownloadStatus::None,
            progress: 0.0,
            error: None,
        });
        self.events.library_changed("download_cancelled");
        Ok(())
    }

    /// Executa um job até o fim. Nunca devolve erro: falha vira status `error` no banco.
    pub async fn process(&self, job: DownloadJob, token: CancellationToken) {
        self.events.download_progress(&job);
        let outcome = self.run_job(&job, &token).await;
        self.active_lock().remove(&job.id);
        match outcome {
            Ok(()) => {}
            Err(e) if e.is_cancelled() || token.is_cancelled() => {
                log::info!("download {} cancelado", job.id);
            }
            Err(e) => {
                log::warn!("download {} falhou: {e}", job.id);
                self.fail(&job, e.to_string()).await;
            }
        }
    }

    async fn fail(&self, job: &DownloadJob, message: String) {
        let id = job.id.clone();
        let res = self
            .db
            .run(move |db| {
                Ok(db.update_download(
                    &id,
                    &DownloadUpdate {
                        status: DownloadStatus::Error,
                        progress: None,
                        error: Some(message),
                    },
                ))
            })
            .await;
        match res {
            Ok(Ok(updated)) => {
                self.events.download_progress(&updated);
                self.events.library_changed("download_failed");
            }
            // job cancelado enquanto falhava: nada a registrar
            Ok(Err(miupy_core::Error::NotFound(_))) => {}
            Ok(Err(e)) => log::error!("não consegui registrar a falha do download: {e}"),
            Err(e) => log::error!("não consegui registrar a falha do download: {e}"),
        }
    }

    async fn run_job(&self, job: &DownloadJob, token: &CancellationToken) -> AppResult<()> {
        let track_id = job.track_id.clone();
        let mut track = self.db.run(move |db| Ok(db.get_track(&track_id)?)).await?;
        let video_id = match (&track.source, &track.source_id) {
            (Source::Youtube, Some(id)) => id.clone(),
            _ => return Err(AppError::invalid("A faixa não tem origem para baixar.")),
        };

        // Progresso: o callback só publica o valor mais recente; uma tarefa à parte grava no
        // banco e emite o evento, no máximo a cada PROGRESS_INTERVAL.
        let (tx, rx) = watch::channel(0.0_f32);
        let stop = CancellationToken::new();
        let reporter = tokio::spawn(report_progress(
            self.db.clone(),
            self.events.clone(),
            job.id.clone(),
            rx,
            stop.clone(),
            token.clone(),
        ));
        let request =
            DownloadRequest::new(video_id.clone(), self.dirs.audio.clone(), track.id.clone())
                .with_progress(Arc::new(move |p| {
                    tx.send_replace(p);
                }))
                .with_cancel(token.clone());
        let result = self.resolver.download_audio(request).await;
        // Espera o relator terminar: nenhuma gravação de progresso pode chegar depois do fim.
        stop.cancel();
        let _ = reporter.await;
        let audio = result?;

        if token.is_cancelled() {
            let _ = std::fs::remove_file(&audio.path);
            return Err(miupy_media::MediaError::Cancelled.into());
        }

        // @req RF-20: MusicBrainz prevalece; título, artista, álbum e ano do vídeo
        // complementam apenas o que faltar. Categorias do YouTube não são gêneros musicais.
        let recording = match self
            .metadata
            .find_recording(
                &track.title,
                track.artist_name.as_deref(),
                Some(audio.duration_ms),
            )
            .await
        {
            Ok(found) => found,
            Err(e) => {
                log::info!("MusicBrainz indisponível para {}: {e}", track.id);
                None
            }
        };
        let video = if recording.is_none()
            || recording
                .as_ref()
                .is_some_and(|r| r.album.is_none() || r.release_year.is_none())
        {
            self.resolver.video_metadata(&video_id).await.ok()
        } else {
            None
        };
        if recording.is_some() || video.is_some() {
            let patch = TrackPatch {
                title: recording
                    .as_ref()
                    .map(|r| r.title.clone())
                    .or_else(|| video.as_ref().and_then(|v| v.track.clone())),
                artist_name: recording
                    .as_ref()
                    .map(|r| Some(r.artist.clone()))
                    .or_else(|| video.as_ref().and_then(|v| v.artist.clone().map(Some))),
                album: recording
                    .as_ref()
                    .and_then(|r| r.album.clone())
                    .or_else(|| video.as_ref().and_then(|v| v.album.clone()))
                    .map(Some),
                genres: recording.as_ref().map(|r| r.genres.clone()),
                release_year: recording
                    .as_ref()
                    .and_then(|r| r.release_year.map(|y| y as i32))
                    .or_else(|| {
                        video
                            .as_ref()
                            .and_then(|v| v.release_year.map(|y| y as i32))
                    })
                    .map(Some),
                mbid: recording.as_ref().map(|r| Some(r.mbid.clone())),
                isrc: recording.as_ref().and_then(|r| r.isrc.clone()).map(Some),
                ..Default::default()
            };
            let id = track.id.clone();
            match self
                .db
                .run(move |db| Ok(db.update_track(&id, &patch)?))
                .await
            {
                Ok(updated) => track = updated,
                Err(e) => log::warn!("metadados do download de {}: {e}", track.id),
            }
        }

        // Capa (miniatura do YouTube). Falha aqui não invalida o download.
        let cover = self.download_cover(&track).await;

        // Tags e capa embutida, em thread de bloqueio. Também não é fatal.
        let update = TagUpdate {
            title: Some(track.title.clone()),
            artist: track.artist_name.clone(),
            album: track.album.clone(),
            genres: Some(track.genres.clone()),
            release_year: track.release_year.and_then(|y| u32::try_from(y).ok()),
            cover: cover.as_deref().and_then(|p| std::fs::read(p).ok()),
        };
        let audio_path = audio.path.clone();
        match tokio::task::spawn_blocking(move || tagger::write_tags(&audio_path, &update)).await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => log::warn!("não consegui gravar as tags de {}: {e}", track.id),
            Err(e) => log::warn!("gravação de tags interrompida: {e}"),
        }

        let file = DownloadedFile {
            file_path: audio.path.to_string_lossy().into_owned(),
            codec: Some(audio.codec.clone()),
            bitrate: audio.bitrate_kbps.map(i64::from),
            cover_path: cover.map(|p| p.to_string_lossy().into_owned()),
            duration_ms: i64::try_from(audio.duration_ms).ok(),
        };
        let track_id = track.id.clone();
        let job_id = job.id.clone();
        let done = self
            .db
            .run(move |db| {
                db.mark_downloaded(&track_id, &file)?;
                Ok(db.get_download(&job_id).ok())
            })
            .await?;
        if let Some(job) = done {
            self.events.download_progress(&job);
        }
        self.events.library_changed("download_done");
        Ok(())
    }

    async fn download_cover(&self, track: &Track) -> Option<PathBuf> {
        let track_id = track.id.clone();
        let url = match self
            .db
            .run(move |db| Ok(db.track_thumbnail_url(&track_id)?))
            .await
        {
            Ok(Some(url)) => url,
            Ok(None) => return None,
            Err(e) => {
                log::warn!("não consegui ler a miniatura de {}: {e}", track.id);
                return None;
            }
        };
        match self
            .metadata
            .download_image(&url, &self.dirs.covers, &track.id)
            .await
        {
            Ok(path) => Some(path),
            Err(e) => {
                log::warn!("não consegui baixar a capa de {}: {e}", track.id);
                None
            }
        }
    }
}

/// Grava e emite o progresso do job, no máximo a cada [`PROGRESS_INTERVAL`]. Se o banco diz que
/// o job não existe mais (cancelado), interrompe o download pelo `token`.
async fn report_progress(
    db: Db,
    events: Arc<dyn Events>,
    job_id: String,
    mut rx: watch::Receiver<f32>,
    stop: CancellationToken,
    token: CancellationToken,
) {
    loop {
        tokio::select! {
            _ = stop.cancelled() => return,
            changed = rx.changed() => {
                if changed.is_err() {
                    return;
                }
            }
        }
        let progress = f64::from(*rx.borrow_and_update());
        let id = job_id.clone();
        let res = db
            .run(move |db| {
                Ok(db.update_download(
                    &id,
                    &DownloadUpdate {
                        status: DownloadStatus::Downloading,
                        progress: Some(progress),
                        error: None,
                    },
                ))
            })
            .await;
        match res {
            Ok(Ok(job)) => events.download_progress(&job),
            Ok(Err(miupy_core::Error::NotFound(_))) => {
                token.cancel();
                return;
            }
            Ok(Err(e)) => log::warn!("não consegui gravar o progresso: {e}"),
            Err(e) => log::warn!("não consegui gravar o progresso: {e}"),
        }
        tokio::select! {
            _ = stop.cancelled() => return,
            _ = tokio::time::sleep(PROGRESS_INTERVAL) => {}
        }
    }
}

/// Apaga o arquivo se existir e for do app; ignora falhas (limpeza de melhor esforço).
pub fn remove_file_quietly(dirs: &Dirs, path: &Path) {
    if dirs.owns(path) {
        let _ = std::fs::remove_file(path);
    }
}

#[cfg(test)]
pub mod testing {
    //! Dublês de `MediaResolver` e `MetadataProvider` para os testes de download e metadados.

    use std::path::Path;
    use std::sync::Mutex;

    use async_trait::async_trait;
    use miupy_media::{
        ArtistDetails, DownloadRequest, DownloadedAudio, LyricsData, MediaError, MediaResolver,
        MetadataProvider, PlaylistInfo, RecordingMatch, SearchResult, error::Result,
    };

    #[derive(Default)]
    pub struct FakeResolver {
        pub results: Mutex<Vec<SearchResult>>,
        pub related: Mutex<Vec<SearchResult>>,
        pub stream_fail: Mutex<Option<MediaError>>,
        pub stream_calls: Mutex<u32>,
        pub stream_expiry: Mutex<Option<u64>>,
        pub always_fail_network: Mutex<bool>,
        pub video: Mutex<Option<miupy_media::VideoMetadata>>,
        /// `Some(erro)` faz o download falhar com esse erro.
        pub fail_with: Mutex<Option<MediaError>>,
        pub downloads: Mutex<Vec<String>>,
    }

    pub fn sr(id: &str, title: &str, channel: &str) -> SearchResult {
        SearchResult {
            video_id: id.into(),
            title: title.into(),
            channel: channel.into(),
            duration_ms: Some(200_000),
            thumbnail_url: Some(format!("https://img.example/{id}.jpg")),
        }
    }

    #[async_trait]
    impl MediaResolver for FakeResolver {
        async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
            if *self.always_fail_network.lock().unwrap() {
                return Err(MediaError::Network("offline".into()));
            }
            if let Some(e) = self.fail_with.lock().unwrap().take() {
                return Err(e);
            }
            let all = self.results.lock().unwrap().clone();
            let query = query.strip_suffix(" music").unwrap_or(query);
            Ok(all
                .into_iter()
                .filter(|r| r.title.to_lowercase().contains(&query.to_lowercase()))
                .take(limit)
                .collect())
        }

        async fn resolve_stream(&self, video_id: &str) -> Result<miupy_media::StreamAudio> {
            *self.stream_calls.lock().unwrap() += 1;
            if let Some(error) = self.stream_fail.lock().unwrap().take() {
                return Err(error);
            }
            Ok(miupy_media::StreamAudio {
                url: format!("https://audio.example/{video_id}.m4a"),
                http_headers: std::collections::BTreeMap::from([(
                    "User-Agent".into(),
                    "miupy-test".into(),
                )]),
                codec: "aac".into(),
                container: "m4a".into(),
                bitrate_kbps: Some(128),
                duration_ms: Some(200_000),
                expires_at: *self.stream_expiry.lock().unwrap(),
            })
        }

        async fn related_tracks(&self, video_id: &str, limit: usize) -> Result<Vec<SearchResult>> {
            if *self.always_fail_network.lock().unwrap() {
                return Err(MediaError::Network("offline".into()));
            }
            if let Some(error) = self.fail_with.lock().unwrap().take() {
                return Err(error);
            }
            Ok(self
                .related
                .lock()
                .unwrap()
                .iter()
                .filter(|r| r.video_id != video_id)
                .take(limit)
                .cloned()
                .collect())
        }

        async fn video_metadata(&self, _video_id: &str) -> Result<miupy_media::VideoMetadata> {
            Ok(self.video.lock().unwrap().clone().unwrap_or_default())
        }

        async fn download_audio(&self, request: DownloadRequest) -> Result<DownloadedAudio> {
            self.downloads
                .lock()
                .unwrap()
                .push(request.video_id.clone());
            if let Some(e) = self.fail_with.lock().unwrap().take() {
                return Err(e);
            }
            if let Some(cb) = &request.progress {
                for p in [0.1, 0.5, 1.0] {
                    cb(p);
                    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                }
            }
            std::fs::create_dir_all(&request.dest_dir).unwrap();
            let path = request.dest_dir.join(format!("{}.m4a", request.file_stem));
            // não é um M4A de verdade: a gravação de tags vai falhar, e isso não é fatal
            std::fs::write(&path, b"nao-e-audio").unwrap();
            Ok(DownloadedAudio {
                path,
                codec: "aac".into(),
                bitrate_kbps: Some(128),
                duration_ms: 201_000,
            })
        }

        async fn resolve_playlist(&self, _url_or_id: &str) -> Result<PlaylistInfo> {
            Err(MediaError::Unavailable("fora do escopo do teste".into()))
        }
    }

    pub struct FakeMetadata {
        pub recording: Mutex<Option<RecordingMatch>>,
        pub artist: Mutex<Option<ArtistDetails>>,
        pub lyrics: Mutex<Option<LyricsData>>,
        /// `Some(erro)` faz a próxima chamada de rede falhar.
        pub fail_with: Mutex<Option<MediaError>>,
        /// release_mbids cuja capa existe; os outros dão `NotFound`.
        pub covers_for: Mutex<Vec<String>>,
        pub cover_attempts: Mutex<Vec<String>>,
        pub network_calls: Mutex<u32>,
    }

    impl Default for FakeMetadata {
        fn default() -> Self {
            FakeMetadata {
                recording: Mutex::new(None),
                artist: Mutex::new(None),
                lyrics: Mutex::new(None),
                fail_with: Mutex::new(None),
                covers_for: Mutex::new(Vec::new()),
                cover_attempts: Mutex::new(Vec::new()),
                network_calls: Mutex::new(0),
            }
        }
    }

    impl FakeMetadata {
        fn call(&self) -> Result<()> {
            *self.network_calls.lock().unwrap() += 1;
            match self.fail_with.lock().unwrap().take() {
                Some(e) => Err(e),
                None => Ok(()),
            }
        }
    }

    #[async_trait]
    impl MetadataProvider for FakeMetadata {
        async fn find_recording(
            &self,
            _title: &str,
            _artist: Option<&str>,
            _duration_ms: Option<u64>,
        ) -> Result<Option<RecordingMatch>> {
            self.call()?;
            Ok(self.recording.lock().unwrap().clone())
        }

        async fn find_artist(&self, _name: &str) -> Result<Option<ArtistDetails>> {
            self.call()?;
            Ok(self.artist.lock().unwrap().clone())
        }

        async fn artist_details(&self, _mbid: &str) -> Result<ArtistDetails> {
            self.call()?;
            self.artist
                .lock()
                .unwrap()
                .clone()
                .ok_or_else(|| MediaError::NotFound("artista".into()))
        }

        async fn download_cover(
            &self,
            release_mbid: &str,
            dest_dir: &Path,
            file_stem: &str,
        ) -> Result<std::path::PathBuf> {
            self.cover_attempts
                .lock()
                .unwrap()
                .push(release_mbid.into());
            self.call()?;
            if !self
                .covers_for
                .lock()
                .unwrap()
                .iter()
                .any(|m| m == release_mbid)
            {
                return Err(MediaError::NotFound("capa".into()));
            }
            std::fs::create_dir_all(dest_dir).unwrap();
            let p = dest_dir.join(format!("{file_stem}.jpg"));
            std::fs::write(&p, b"jpg").unwrap();
            Ok(p)
        }

        async fn lyrics(
            &self,
            _title: &str,
            _artist: &str,
            _album: Option<&str>,
            _duration_ms: Option<u64>,
        ) -> Result<Option<LyricsData>> {
            self.call()?;
            Ok(self.lyrics.lock().unwrap().clone())
        }

        async fn download_image(
            &self,
            _url: &str,
            dest_dir: &Path,
            file_stem: &str,
        ) -> Result<std::path::PathBuf> {
            self.call()?;
            std::fs::create_dir_all(dest_dir).unwrap();
            let p = dest_dir.join(format!("{file_stem}.jpg"));
            std::fs::write(&p, b"jpg").unwrap();
            Ok(p)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::testing::*;
    use super::*;
    use crate::events::testing::RecordingEvents;
    use miupy_core::{Database, NewTrack};
    use miupy_media::MediaError;

    struct Fx {
        dl: Arc<Downloads>,
        resolver: Arc<FakeResolver>,
        events: Arc<RecordingEvents>,
        db: Db,
        dirs: Dirs,
        _tmp: tempfile::TempDir,
    }

    fn fx() -> Fx {
        let tmp = tempfile::tempdir().unwrap();
        let dirs = Dirs::new(tmp.path());
        dirs.create_all().unwrap();
        let db = Db::new(Database::open_in_memory().unwrap());
        let resolver = Arc::new(FakeResolver::default());
        let events = Arc::new(RecordingEvents::default());
        let dl = Downloads::new(
            db.clone(),
            resolver.clone(),
            Arc::new(FakeMetadata::default()),
            events.clone(),
            dirs.clone(),
        );
        Fx {
            dl,
            resolver,
            events,
            db,
            dirs,
            _tmp: tmp,
        }
    }

    fn add(f: &Fx, vid: &str) -> Track {
        f.db.with(|db| {
            db.upsert_track(&NewTrack::youtube(
                vid,
                format!("Música {vid}"),
                Some("Artista".into()),
                200_000,
                Some(format!("https://img.example/{vid}.jpg")),
            ))
        })
        .unwrap()
        .track
    }

    fn enqueue(f: &Fx, ids: &[&Track]) -> Vec<DownloadJob> {
        let ids: Vec<String> = ids.iter().map(|t| t.id.clone()).collect();
        f.db.with(|db| db.enqueue_downloads(&ids)).unwrap()
    }

    // @test RF-10
    #[tokio::test]
    async fn baixa_grava_capa_e_marca_a_faixa() {
        let f = fx();
        let t = add(&f, "vidA");
        enqueue(&f, &[&t]);
        f.dl.run_until_idle().await;

        let t = f.db.with(|db| db.get_track(&t.id)).unwrap();
        assert_eq!(t.download_status, DownloadStatus::Done);
        let expected = f.dirs.audio.join(format!("{}.m4a", t.id));
        assert_eq!(t.file_path.as_deref(), Some(expected.to_str().unwrap()));
        assert!(expected.is_file());
        assert_eq!(t.codec.as_deref(), Some("aac"));
        assert_eq!(t.bitrate, Some(128));
        assert_eq!(t.duration_ms, 201_000);
        let cover = t.cover_path.expect("capa gravada");
        assert!(Path::new(&cover).starts_with(&f.dirs.covers));

        let jobs = f.db.with(|db| db.list_downloads()).unwrap();
        assert_eq!(jobs[0].status, DownloadStatus::Done);
        assert_eq!(jobs[0].progress, 1.0);

        // eventos: começou, teve progresso, terminou; e a biblioteca mudou
        let ev = f.events.downloads.lock().unwrap().clone();
        assert_eq!(ev.first().unwrap().status, DownloadStatus::Downloading);
        assert_eq!(ev.last().unwrap().status, DownloadStatus::Done);
        assert!(ev.iter().any(|j| j.progress > 0.0
            && j.progress < 1.0
            && j.status == DownloadStatus::Downloading));
        assert!(
            f.events
                .library
                .lock()
                .unwrap()
                .contains(&"download_done".to_string())
        );
        assert!(f.dl.active_lock().is_empty());
    }

    // @test RF-10 RF-11
    #[tokio::test]
    async fn processa_varios_jobs_em_ordem() {
        let f = fx();
        let a = add(&f, "vidA");
        let b = add(&f, "vidB");
        enqueue(&f, &[&a, &b]);
        f.dl.run_until_idle().await;
        assert_eq!(*f.resolver.downloads.lock().unwrap(), vec!["vidA", "vidB"]);
        let all = f.db.with(|db| db.list_downloads()).unwrap();
        assert!(all.iter().all(|j| j.status == DownloadStatus::Done));
    }

    // @test RF-10
    #[tokio::test]
    async fn falha_vira_status_error_com_a_mensagem() {
        let f = fx();
        let t = add(&f, "vidA");
        enqueue(&f, &[&t]);
        *f.resolver.fail_with.lock().unwrap() = Some(MediaError::Network("sem rede".into()));
        f.dl.run_until_idle().await;
        let job = &f.db.with(|db| db.list_downloads()).unwrap()[0];
        assert_eq!(job.status, DownloadStatus::Error);
        assert!(job.error.as_deref().unwrap().contains("sem rede"));
        let t = f.db.with(|db| db.get_track(&t.id)).unwrap();
        assert_eq!(t.download_status, DownloadStatus::Error);
        assert!(t.file_path.is_none());
    }

    // @test RF-10
    #[tokio::test]
    async fn yt_dlp_ausente_vira_error_com_instrucao_de_instalacao() {
        let f = fx();
        let t = add(&f, "vidA");
        enqueue(&f, &[&t]);
        *f.resolver.fail_with.lock().unwrap() = Some(MediaError::Unavailable(
            "yt-dlp não encontrado. Instale com `pipx install yt-dlp`".into(),
        ));
        f.dl.run_until_idle().await;
        let job = &f.db.with(|db| db.list_downloads()).unwrap()[0];
        assert_eq!(job.status, DownloadStatus::Error);
        assert!(
            job.error
                .as_deref()
                .unwrap()
                .contains("pipx install yt-dlp")
        );
    }

    // @test RF-10
    #[tokio::test]
    async fn cancelamento_nao_e_erro_para_o_usuario() {
        let f = fx();
        let t = add(&f, "vidA");
        let jobs = enqueue(&f, &[&t]);
        *f.resolver.fail_with.lock().unwrap() = Some(MediaError::Cancelled);
        f.dl.run_until_idle().await;
        // o job fica como estava (downloading), sem virar `error`
        let job = f.db.with(|db| db.get_download(&jobs[0].id)).unwrap();
        assert_ne!(job.status, DownloadStatus::Error);
        assert!(job.error.is_none());
    }

    // @test RF-10
    #[tokio::test]
    async fn cancelar_remove_o_job_e_avisa_o_frontend() {
        let f = fx();
        let t = add(&f, "vidA");
        let jobs = enqueue(&f, &[&t]);
        f.dl.cancel(&jobs[0].id).unwrap();
        assert!(f.db.with(|db| db.list_downloads()).unwrap().is_empty());
        let t = f.db.with(|db| db.get_track(&t.id)).unwrap();
        assert_eq!(t.download_status, DownloadStatus::None);
        let last = f.events.downloads.lock().unwrap().last().cloned().unwrap();
        assert_eq!(last.status, DownloadStatus::None);
        assert_eq!(last.id, jobs[0].id);
        // nada mais para baixar
        f.dl.run_until_idle().await;
        assert!(f.resolver.downloads.lock().unwrap().is_empty());
        assert_eq!(f.dl.cancel("nao-existe").unwrap_err().code(), "not_found");
    }

    // @test RF-10
    #[tokio::test]
    async fn job_apagado_no_meio_cancela_o_download() {
        // O relator percebe `not_found` ao gravar o progresso e cancela o token.
        let f = fx();
        let t = add(&f, "vidA");
        let jobs = enqueue(&f, &[&t]);
        let (job, token) = f.dl.claim_next().await.unwrap().unwrap();
        f.db.with(|db| db.cancel_download(&jobs[0].id)).unwrap();
        let (tx, rx) = watch::channel(0.0_f32);
        let stop = CancellationToken::new();
        let h = tokio::spawn(report_progress(
            f.db.clone(),
            f.events.clone(),
            job.id.clone(),
            rx,
            stop,
            token.clone(),
        ));
        tx.send_replace(0.4);
        h.await.unwrap();
        assert!(token.is_cancelled());
    }

    // @test RF-10
    #[tokio::test]
    async fn requeue_no_boot_devolve_downloading_para_a_fila() {
        let f = fx();
        let t = add(&f, "vidA");
        enqueue(&f, &[&t]);
        f.dl.claim_next().await.unwrap().unwrap(); // vira `downloading` e o "app fecha"
        assert_eq!(
            f.db.with(|db| db.requeue_interrupted_downloads()).unwrap(),
            1
        );
        f.dl.run_until_idle().await;
        let t = f.db.with(|db| db.get_track(&t.id)).unwrap();
        assert_eq!(t.download_status, DownloadStatus::Done);
    }
}
