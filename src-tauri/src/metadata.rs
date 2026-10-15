//! Enriquecimento de metadados (RF-12): MusicBrainz, Cover Art Archive e LRCLIB, via
//! `MetadataProvider`. Falha de rede nunca derruba a leitura local.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use miupy_core::{ArtistInfo, ArtistPatch, Lyrics, Track, TrackPatch};
use miupy_media::{MediaError, MediaResolver, MetadataProvider, TagUpdate, tagger};

use crate::db::Db;
use crate::dto::MetadataRefreshStatus;
use crate::error::AppResult;
use crate::events::Events;
use crate::paths::Dirs;

/// Limite de espera por uma leitura enriquecida (artista, letra): offline não pode travar a tela.
const READ_TIMEOUT: Duration = Duration::from_secs(12);
/// Depois de uma tentativa (com ou sem sucesso), não repete a mesma consulta por este tempo.
const RETRY_AFTER: Duration = Duration::from_secs(300);
/// Falha temporária pode ser consultada novamente em breve, sem repetir imediatamente.
const TEMPORARY_RETRY_AFTER: Duration = Duration::from_secs(10);

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub struct MetadataService {
    db: Db,
    provider: Arc<dyn MetadataProvider>,
    resolver: Arc<dyn MediaResolver>,
    dirs: Dirs,
    events: Arc<dyn Events>,
    attempts: Mutex<HashMap<String, Instant>>,
    batch_status: Mutex<MetadataRefreshStatus>,
}

impl MetadataService {
    pub fn new(
        db: Db,
        provider: Arc<dyn MetadataProvider>,
        resolver: Arc<dyn MediaResolver>,
        dirs: Dirs,
        events: Arc<dyn Events>,
    ) -> Self {
        MetadataService {
            db,
            provider,
            resolver,
            dirs,
            events,
            attempts: Mutex::new(HashMap::new()),
            batch_status: Mutex::new(MetadataRefreshStatus::default()),
        }
    }

    // @req RF-32
    pub fn refresh_status(&self) -> MetadataRefreshStatus {
        self.batch_status
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    fn update_batch_status(&self, update: impl FnOnce(&mut MetadataRefreshStatus)) {
        let status = {
            let mut status = self.batch_status.lock().unwrap_or_else(|e| e.into_inner());
            update(&mut status);
            status.clone()
        };
        self.events.metadata_progress(&status);
    }

    // @req RF-12 RF-20 RF-32
    pub fn enqueue_library_refresh(self: &Arc<Self>, only_missing: bool) -> AppResult<usize> {
        let previous = {
            let mut status = self.batch_status.lock().unwrap_or_else(|e| e.into_inner());
            if status.running {
                return Err(crate::error::AppError::new(
                    "conflict",
                    "Um reprocessamento já está em andamento.",
                ));
            }
            let previous = status.clone();
            status.running = true;
            previous
        };
        let tracks = match self
            .db
            .with(|db| db.tracks_for_metadata_refresh(only_missing))
        {
            Ok(t) => t,
            Err(e) => {
                *self.batch_status.lock().unwrap_or_else(|e| e.into_inner()) = previous;
                return Err(e.into());
            }
        };
        let total = tracks.len();
        self.update_batch_status(|status| {
            *status = MetadataRefreshStatus {
                running: true,
                total,
                started_at: Some(now_ms()),
                ..MetadataRefreshStatus::default()
            };
        });
        let this = self.clone();
        tauri::async_runtime::spawn(async move {
            for (i, t) in tracks.into_iter().enumerate() {
                this.batch_status
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .current_track_id = Some(t.id.clone());
                let mut failed = false;
                if let Err(e) = this.refresh_track(&t.id).await {
                    log::warn!("reprocessamento de {}: {e}", t.id);
                    failed = true;
                }
                this.update_batch_status(|status| {
                    status.done = i + 1;
                    status.failed += usize::from(failed);
                });
            }
            this.events.library_changed("metadata_batch_finished");
            this.update_batch_status(|status| {
                status.running = false;
                status.current_track_id = None;
                status.finished_at = Some(now_ms());
            });
        });
        Ok(total)
    }

    /// `true` se vale tentar a consulta `key` agora; registra a tentativa.
    fn should_try(&self, key: String) -> bool {
        let mut attempts = self.attempts.lock().unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();
        match attempts.get(&key) {
            Some(at) if now.duration_since(*at) < RETRY_AFTER => false,
            _ => {
                attempts.insert(key, now);
                true
            }
        }
    }

    // @req RF-12 RF-27
    /// Falha temporária (rede, 503/429, tempo esgotado): permite nova tentativa em
    /// `TEMPORARY_RETRY_AFTER` em vez de `RETRY_AFTER`.
    fn retry_soon(&self, key: &str) {
        let now = Instant::now();
        // `checked_sub`: logo depois do boot o `Instant` pode ser menor que o intervalo.
        let Some(at) = now.checked_sub(RETRY_AFTER - TEMPORARY_RETRY_AFTER) else {
            self.attempts
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .remove(key);
            return;
        };
        let mut attempts = self.attempts.lock().unwrap_or_else(|e| e.into_inner());
        attempts.insert(key.to_owned(), at);
    }

    /// Só erro de rede (inclui 503/429) é temporário; "não encontrado", entrada inválida e
    /// erros locais mantêm o intervalo longo marcado por `should_try`.
    fn retry_soon_if_temporary(&self, key: &str, code: &str) {
        if code == "network" {
            self.retry_soon(key);
        }
    }

    // @req RF-12
    /// Procura a gravação no MusicBrainz e atualiza título, artista, álbum, capa e letra.
    /// Aqui a rede é o objetivo: erro de rede volta ao usuário. Sem correspondência, devolve a
    /// faixa como estava.
    pub async fn refresh_track(&self, track_id: &str) -> AppResult<Track> {
        let id = track_id.to_owned();
        let track = self.db.run(move |db| Ok(db.get_track(&id)?)).await?;
        let duration = u64::try_from(track.duration_ms).ok().filter(|d| *d > 0);

        let rec = self
            .provider
            .find_recording(&track.title, track.artist_name.as_deref(), duration)
            .await?;
        let Some(rec) = rec else {
            // @req RF-20: sem correspondência no MusicBrainz, usa apenas os campos
            // musicais do vídeo; suas categorias nunca viram gênero.
            let Some(source_id) = &track.source_id else {
                return Ok(track);
            };
            let video = match self.resolver.video_metadata(source_id).await {
                Ok(v) => v,
                Err(_) => return Ok(track),
            };
            let patch = TrackPatch {
                title: video.track,
                artist_name: video.artist.map(Some),
                album: video.album.map(Some),
                release_year: video.release_year.map(|year| Some(year as i32)),
                ..Default::default()
            };
            let id = track.id.clone();
            let updated = self
                .db
                .run(move |db| Ok(db.update_track(&id, &patch)?))
                .await?;
            self.write_file_tags(&updated, None).await;
            self.events.library_changed("metadata_refreshed");
            return Ok(updated);
        };

        // Capa: tenta os releases candidatos em ordem; o primeiro que tiver capa vale.
        let mut mbids = rec.release_mbids.clone();
        if mbids.is_empty() {
            mbids.extend(rec.release_mbid.clone());
        }
        let key = crate::paths::album_key(&rec);
        let mut cover = None;
        for mbid in mbids {
            let album_dir = self.dirs.covers.join("albuns");
            let cached = album_dir.join(format!("{key}.webp"));
            if cached.is_file() {
                cover = Some(cached);
                break;
            }
            match self.provider.download_cover(&mbid, &album_dir, &key).await {
                Ok(path) => {
                    // @req RNF-12
                    cover = crate::paths::normalize_webp(&path).ok();
                    break;
                }
                Err(MediaError::NotFound(_)) => continue,
                Err(e) => {
                    log::warn!("capa de {}: {e}", track.id);
                    break;
                }
            }
        }

        // Letra (LRCLIB); falhar não desfaz o resto.
        let lyrics = match self
            .provider
            .lyrics(
                &rec.title,
                &rec.artist,
                rec.album.as_deref().or(track.album.as_deref()),
                duration.or(rec.duration_ms),
            )
            .await
        {
            Ok(l) => l.filter(|l| !l.instrumental && (l.plain.is_some() || l.synced.is_some())),
            Err(e) => {
                log::warn!("letra de {}: {e}", track.id);
                None
            }
        };

        // @req RF-20: MusicBrainz é a fonte principal; o vídeo só complementa campos ausentes.
        let video = if let Some(source_id) = &track.source_id {
            self.resolver.video_metadata(source_id).await.ok()
        } else {
            None
        };
        let patch = TrackPatch {
            title: Some(rec.title.clone())
                .filter(|t| !t.trim().is_empty())
                .or_else(|| video.as_ref().and_then(|v| v.track.clone())),
            artist_name: Some(rec.artist.clone())
                .filter(|a| !a.trim().is_empty())
                .or_else(|| video.as_ref().and_then(|v| v.artist.clone()))
                .filter(|a| !a.trim().is_empty())
                .map(Some),
            album: rec
                .album
                .clone()
                .or_else(|| video.as_ref().and_then(|v| v.album.clone()))
                .map(Some),
            duration_ms: if track.duration_ms <= 0 {
                rec.duration_ms.and_then(|d| i64::try_from(d).ok())
            } else {
                None
            },
            cover_path: cover
                .as_ref()
                .map(|p| Some(p.to_string_lossy().into_owned())),
            genres: Some(rec.genres.clone()),
            release_year: rec
                .release_year
                .map(|y| y as i32)
                .or_else(|| {
                    video
                        .as_ref()
                        .and_then(|v| v.release_year.map(|y| y as i32))
                })
                .map(Some),
            mbid: Some(Some(rec.mbid.clone())),
            isrc: rec.isrc.clone().map(Some),
        };
        let artist_mbid = rec.artist_mbid.clone();
        let artist_name = rec.artist.clone();
        let tid = track.id.clone();
        let updated = self
            .db
            .run(move |db| {
                let updated = db.update_track(&tid, &patch)?;
                if let Some(l) = lyrics {
                    db.set_lyrics(&Lyrics {
                        track_id: tid.clone(),
                        plain: l.plain,
                        synced: l.synced,
                    })?;
                }
                let mut updated = updated;
                if let Some(aid) = &updated.artist_id {
                    // O núcleo reaproveita o artista de mesmo nome (sem diferenciar caixa), então
                    // "daft punk" continuaria assim; aqui ele ganha a grafia do MusicBrainz.
                    let rename = (updated.artist_name.as_deref() != Some(artist_name.as_str()))
                        .then(|| artist_name.clone());
                    if rename.is_some() || artist_mbid.is_some() {
                        let patch = ArtistPatch {
                            name: rename,
                            mbid: artist_mbid.map(Some),
                            ..Default::default()
                        };
                        match db.update_artist(aid, &patch) {
                            Ok(_) => updated = db.get_track(&tid)?,
                            Err(e) => log::warn!("artista de {tid}: {e}"),
                        }
                    }
                }
                db.log_action("metadata_refreshed", Some("track"), Some(&tid), None)?;
                Ok(updated)
            })
            .await?;

        self.write_file_tags(&updated, cover.as_deref()).await;
        self.events.library_changed("metadata_refreshed");
        Ok(updated)
    }

    /// Regrava as tags do arquivo com os dados novos (melhor esforço).
    async fn write_file_tags(&self, track: &Track, cover: Option<&std::path::Path>) {
        let Some(file) = track.file_path.clone().map(std::path::PathBuf::from) else {
            return;
        };
        if !file.is_file() {
            return;
        }
        let update = TagUpdate {
            title: Some(track.title.clone()),
            artist: track.artist_name.clone(),
            album: track.album.clone(),
            genres: Some(track.genres.clone()),
            release_year: track.release_year.and_then(|year| u32::try_from(year).ok()),
            // @req RF-12: tags de M4A continuam em JPEG.
            cover: cover.and_then(crate::paths::jpeg_for_tags),
        };
        match tokio::task::spawn_blocking(move || tagger::write_tags(&file, &update)).await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => log::warn!("tags de {}: {e}", track.id),
            Err(e) => log::warn!("tags de {}: {e}", track.id),
        }
    }

    // @req RF-12
    /// Dados do artista no banco; se faltar o que o MusicBrainz sabe, tenta enriquecer e
    /// persistir. Sem rede (ou com erro), devolve o que há localmente.
    pub async fn get_artist(&self, artist_id: &str) -> AppResult<ArtistInfo> {
        let id = artist_id.to_owned();
        let mut info = self.db.run(move |db| Ok(db.get_artist_info(&id)?)).await?;
        // @req RF-27: sem MBID, encontre o artista antes de consultar a foto.
        let incomplete = info.mbid.is_none() || (info.country.is_none() && info.genres.is_empty());
        if incomplete && self.should_try(format!("artist:{}", info.id)) {
            match tokio::time::timeout(READ_TIMEOUT, self.enrich_artist(&info)).await {
                Ok(Ok(Some(updated))) => info = updated,
                Ok(Ok(None)) => {}
                Ok(Err(e)) => {
                    log::info!("sem enriquecimento para o artista {}: {e}", info.name);
                    self.retry_soon_if_temporary(&format!("artist:{}", info.id), e.code());
                }
                Err(_) => {
                    log::info!("tempo esgotado ao enriquecer o artista {}", info.name);
                    self.retry_soon(&format!("artist:{}", info.id));
                }
            }
        }
        let photo = if info.image_path.is_none()
            && info.mbid.is_some()
            && self.should_try(format!("artist-photo:{}", info.id))
        {
            match tokio::time::timeout(
                READ_TIMEOUT,
                self.provider.artist_photo(info.mbid.as_deref()),
            )
            .await
            {
                Ok(Ok(found)) => found,
                Ok(Err(e)) => {
                    log::info!("sem foto para o artista {}: {e}", info.name);
                    self.retry_soon_if_temporary(&format!("artist-photo:{}", info.id), e.code());
                    None
                }
                Err(_) => {
                    log::info!("tempo esgotado ao buscar a foto de {}", info.name);
                    self.retry_soon(&format!("artist-photo:{}", info.id));
                    None
                }
            }
        } else {
            None
        };
        if let Some(photo) = photo
            && let Ok(img) = image::load_from_memory(&photo.bytes)
                .inspect_err(|e| log::warn!("foto de {} ilegível: {e}", info.name))
        {
            let dir = self.dirs.covers.join("artistas");
            // @req RF-27 RNF-12
            let path = dir.join(format!("{}.webp", info.id));
            if std::fs::create_dir_all(&dir).is_ok()
                && crate::paths::write_webp(&crate::paths::compact_image(img, false), &path).is_ok()
            {
                let id = info.id.clone();
                let p = path.to_string_lossy().into_owned();
                let credit = photo.credit;
                if let Ok(updated) = self
                    .db
                    .run(move |db| {
                        db.set_artist_image(&id, Some(&p), Some(&credit))?;
                        Ok(db.get_artist_info(&id)?)
                    })
                    .await
                {
                    self.events.library_changed("artist_photo_updated");
                    return Ok(updated);
                }
            }
        }
        Ok(info)
    }

    async fn enrich_artist(&self, info: &ArtistInfo) -> AppResult<Option<ArtistInfo>> {
        let details = match &info.mbid {
            Some(mbid) => Some(self.provider.artist_details(mbid).await?),
            None => self.provider.find_artist(&info.name).await?,
        };
        let Some(d) = details else {
            return Ok(None);
        };
        let image = match (&d.image_url, &info.image_path) {
            (Some(url), None) => {
                let dir = self.dirs.covers.join("artistas");
                match self.provider.download_image(url, &dir, &info.id).await {
                    Ok(p) => crate::paths::normalize_webp(&p)
                        .ok()
                        .map(|p| p.to_string_lossy().into_owned()),
                    Err(e) => {
                        log::warn!("foto do artista {}: {e}", info.name);
                        None
                    }
                }
            }
            _ => None,
        };
        let patch = ArtistPatch {
            mbid: Some(Some(d.mbid)),
            country: d.country.map(Some),
            genres: Some(d.genres).filter(|g| !g.is_empty()),
            bio: d.bio.map(Some),
            image_path: image.map(Some),
            ..Default::default()
        };
        let id = info.id.clone();
        let updated = self
            .db
            .run(move |db| Ok(db.update_artist(&id, &patch)?))
            .await?;
        self.events.library_changed("artist_updated");
        Ok(Some(updated))
    }

    // @req RF-12
    /// Letra do banco; se não houver, busca no LRCLIB e grava. Falha de rede devolve vazio.
    pub async fn get_lyrics(&self, track_id: &str) -> AppResult<Lyrics> {
        let id = track_id.to_owned();
        let (track, stored) = self
            .db
            .run(move |db| Ok((db.get_track(&id)?, db.get_lyrics(&id)?)))
            .await?;
        if stored.plain.is_some() || stored.synced.is_some() {
            return Ok(stored);
        }
        if !self.should_try(format!("lyrics:{}", track.id)) {
            return Ok(stored);
        }
        let duration = u64::try_from(track.duration_ms).ok().filter(|d| *d > 0);
        let fetched = tokio::time::timeout(
            READ_TIMEOUT,
            self.provider.lyrics(
                &track.title,
                track.artist_name.as_deref().unwrap_or_default(),
                track.album.as_deref(),
                duration,
            ),
        )
        .await;
        let data = match fetched {
            Ok(Ok(Some(d))) if !d.instrumental && (d.plain.is_some() || d.synced.is_some()) => d,
            Ok(Ok(_)) => return Ok(stored),
            Ok(Err(e)) => {
                log::info!("sem letra para {}: {e}", track.id);
                self.retry_soon_if_temporary(&format!("lyrics:{}", track.id), e.code());
                return Ok(stored);
            }
            Err(_) => {
                log::info!("tempo esgotado buscando a letra de {}", track.id);
                self.retry_soon(&format!("lyrics:{}", track.id));
                return Ok(stored);
            }
        };
        let lyrics = Lyrics {
            track_id: track.id.clone(),
            plain: data.plain,
            synced: data.synced,
        };
        match self.db.run(move |db| Ok(db.set_lyrics(&lyrics)?)).await {
            Ok(saved) => Ok(saved),
            Err(e) => {
                log::warn!("não consegui gravar a letra: {e}");
                Ok(stored)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::downloads::testing::FakeMetadata;
    use crate::events::testing::RecordingEvents;
    use miupy_core::{Database, NewTrack};
    use miupy_media::{ArtistDetails, LyricsData, RecordingMatch};

    struct Fx {
        svc: MetadataService,
        fake: Arc<FakeMetadata>,
        resolver: Arc<crate::downloads::testing::FakeResolver>,
        db: Db,
        events: Arc<RecordingEvents>,
        dirs: Dirs,
        _tmp: tempfile::TempDir,
    }

    fn fx() -> Fx {
        let tmp = tempfile::tempdir().unwrap();
        let dirs = Dirs::new(tmp.path());
        dirs.create_all().unwrap();
        let db = Db::new(Database::open_in_memory().unwrap());
        let fake = Arc::new(FakeMetadata::default());
        let resolver = Arc::new(crate::downloads::testing::FakeResolver::default());
        let events = Arc::new(RecordingEvents::default());
        Fx {
            svc: MetadataService::new(
                db.clone(),
                fake.clone(),
                resolver.clone(),
                dirs.clone(),
                events.clone(),
            ),
            fake,
            resolver,
            db,
            events,
            dirs,
            _tmp: tmp,
        }
    }

    fn track(f: &Fx) -> Track {
        f.db.with(|db| {
            db.upsert_track(&NewTrack::youtube(
                "vid1",
                "get lucky",
                Some("daft punk".into()),
                0,
                None,
            ))
        })
        .unwrap()
        .track
    }

    fn rec() -> RecordingMatch {
        RecordingMatch {
            mbid: "rec-1".into(),
            title: "Get Lucky".into(),
            artist: "Daft Punk".into(),
            artist_mbid: Some("art-1".into()),
            album: Some("Random Access Memories".into()),
            release_mbid: Some("rel-1".into()),
            release_group_mbid: None,
            release_mbids: vec!["rel-1".into(), "rel-2".into(), "rel-3".into()],
            duration_ms: Some(369_000),
            score: 100,
            genres: Vec::new(),
            release_year: None,
            isrc: None,
        }
    }

    // @test RF-32
    #[tokio::test]
    async fn lote_publica_inicio_contagem_falhas_e_fim() {
        let f = fx();
        track(&f);
        f.db.with(|db| db.upsert_track(&NewTrack::youtube("vid2", "Outra", None, 1000, None)))
            .unwrap();
        *f.fake.fail_with.lock().unwrap() = Some(MediaError::Network("sem rede".into()));
        let svc = Arc::new(f.svc);
        assert_eq!(svc.refresh_status(), MetadataRefreshStatus::default());
        assert_eq!(svc.enqueue_library_refresh(false).unwrap(), 2);
        let start = svc.refresh_status();
        assert!(start.running);
        assert_eq!((start.done, start.total, start.failed), (0, 2, 0));
        assert!(start.started_at.is_some());
        assert!(start.finished_at.is_none());
        tokio::time::timeout(Duration::from_secs(3), async {
            while f.events.metadata.lock().unwrap().len() < 4 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let end = svc.refresh_status();
        assert_eq!((end.done, end.total, end.failed), (2, 2, 1));
        assert_eq!(end.current_track_id, None);
        assert!(end.finished_at.unwrap() >= end.started_at.unwrap());
        let events = f.events.metadata.lock().unwrap();
        assert_eq!(events.len(), 4);
        assert_eq!((events[0].running, events[0].done), (true, 0));
        assert_eq!((events[1].done, events[1].failed), (1, 1));
        assert_eq!((events[2].done, events[2].failed), (2, 1));
        assert!(!events[3].running);
    }

    // @test RF-32
    #[tokio::test]
    async fn segundo_pedido_devolve_conflict_sem_apagar_estado() {
        let f = fx();
        track(&f);
        let svc = Arc::new(f.svc);
        svc.enqueue_library_refresh(false).unwrap();
        let status = svc.refresh_status();
        assert_eq!(
            svc.enqueue_library_refresh(false).unwrap_err().code(),
            "conflict"
        );
        assert_eq!(svc.refresh_status(), status);
    }

    // @test RF-20
    #[tokio::test]
    async fn musicbrainz_prevalece_e_persiste_identificadores() {
        let f = fx();
        let t = track(&f);
        let mut mb = rec();
        mb.genres = vec!["disco".into()];
        mb.release_year = Some(2013);
        mb.isrc = Some("USABC1300001".into());
        *f.fake.recording.lock().unwrap() = Some(mb);
        *f.resolver.video.lock().unwrap() = Some(miupy_media::VideoMetadata {
            artist: Some("Outro".into()),
            track: Some("Vídeo".into()),
            album: Some("Outro álbum".into()),
            release_year: Some(2020),
            genres: vec!["film & animation".into()],
        });
        let saved = f.svc.refresh_track(&t.id).await.unwrap();
        assert_eq!(saved.genres, vec!["disco"]);
        assert_eq!(saved.release_year, Some(2013));
        assert_eq!(saved.title, "Get Lucky");
        assert_eq!(
            f.db.with(|db| db.get_track(&t.id)).unwrap().genres,
            vec!["disco"]
        );
    }

    // @test RF-20
    #[tokio::test]
    async fn video_complementa_sem_virar_genero() {
        let f = fx();
        let t = track(&f);
        *f.resolver.video.lock().unwrap() = Some(miupy_media::VideoMetadata {
            artist: Some("Artista".into()),
            track: Some("Título".into()),
            album: Some("Álbum".into()),
            release_year: Some(2022),
            genres: vec!["music".into()],
        });
        let saved = f.svc.refresh_track(&t.id).await.unwrap();
        assert_eq!(saved.release_year, Some(2022));
        assert_eq!(saved.album.as_deref(), Some("Álbum"));
        assert!(saved.genres.is_empty());
    }

    // @test RF-12
    #[tokio::test]
    async fn refresh_atualiza_faixa_capa_letra_e_artista() {
        let f = fx();
        let t = track(&f);
        *f.fake.recording.lock().unwrap() = Some(rec());
        *f.fake.covers_for.lock().unwrap() = vec!["rel-2".into()];
        *f.fake.lyrics.lock().unwrap() = Some(LyricsData {
            plain: Some("We've come too far".into()),
            synced: Some("[00:01.00] We've come too far".into()),
            instrumental: false,
        });
        let updated = f.svc.refresh_track(&t.id).await.unwrap();
        assert_eq!(updated.title, "Get Lucky");
        assert_eq!(updated.artist_name.as_deref(), Some("Daft Punk"));
        assert_eq!(updated.album.as_deref(), Some("Random Access Memories"));
        assert_eq!(updated.duration_ms, 369_000);
        // a capa foi procurada em ordem e parou no primeiro release que tinha
        assert_eq!(
            *f.fake.cover_attempts.lock().unwrap(),
            vec!["rel-1", "rel-2"]
        );
        let cover = updated.cover_path.expect("capa");
        assert!(std::path::Path::new(&cover).starts_with(&f.dirs.covers));
        let l = f.db.with(|db| db.get_lyrics(&t.id)).unwrap();
        assert_eq!(l.plain.as_deref(), Some("We've come too far"));
        assert!(l.synced.is_some());
        let artist =
            f.db.with(|db| db.get_artist_info(updated.artist_id.as_ref().unwrap()))
                .unwrap();
        assert_eq!(artist.mbid.as_deref(), Some("art-1"));
        assert!(
            f.events
                .library
                .lock()
                .unwrap()
                .contains(&"metadata_refreshed".to_string())
        );
    }

    // @test RF-12
    #[tokio::test]
    async fn refresh_sem_correspondencia_devolve_a_faixa_igual() {
        let f = fx();
        let t = track(&f);
        let same = f.svc.refresh_track(&t.id).await.unwrap();
        assert_eq!(same, t);
    }

    // @test RF-12
    #[tokio::test]
    async fn refresh_com_rede_fora_devolve_erro_network() {
        let f = fx();
        let t = track(&f);
        *f.fake.fail_with.lock().unwrap() = Some(MediaError::Network("sem rede".into()));
        let err = f.svc.refresh_track(&t.id).await.unwrap_err();
        assert_eq!(err.code(), "network");
        assert_eq!(f.db.with(|db| db.get_track(&t.id)).unwrap(), t);
    }

    // @test RF-12
    #[tokio::test]
    async fn letra_do_banco_vem_primeiro_sem_rede() {
        let f = fx();
        let t = track(&f);
        f.db.with(|db| {
            db.set_lyrics(&Lyrics {
                track_id: t.id.clone(),
                plain: Some("local".into()),
                synced: None,
            })
        })
        .unwrap();
        let l = f.svc.get_lyrics(&t.id).await.unwrap();
        assert_eq!(l.plain.as_deref(), Some("local"));
        assert_eq!(*f.fake.network_calls.lock().unwrap(), 0);
    }

    // @test RF-12
    #[tokio::test]
    async fn letra_ausente_busca_e_grava_e_falha_de_rede_devolve_vazio() {
        let f = fx();
        let t = track(&f);
        *f.fake.fail_with.lock().unwrap() = Some(MediaError::Network("offline".into()));
        let l = f.svc.get_lyrics(&t.id).await.unwrap();
        assert!(l.plain.is_none() && l.synced.is_none());
        // Falha temporária só mantém um intervalo curto; ausência real mantém o longo.
        let calls = *f.fake.network_calls.lock().unwrap();
        let key = format!("lyrics:{}", t.id);
        let age = {
            let attempts = f.svc.attempts.lock().unwrap();
            attempts.get(&key).unwrap().elapsed()
        };
        assert!(age >= RETRY_AFTER - Duration::from_secs(10));
        f.svc.get_lyrics(&t.id).await.unwrap();
        assert_eq!(*f.fake.network_calls.lock().unwrap(), calls);

        // com outra faixa e rede ok, a letra é buscada e persistida
        let t2 =
            f.db.with(|db| db.upsert_track(&NewTrack::youtube("vid2", "Outra", None, 1000, None)))
                .unwrap()
                .track;
        *f.fake.lyrics.lock().unwrap() = Some(LyricsData {
            plain: Some("la la".into()),
            synced: None,
            instrumental: false,
        });
        let l = f.svc.get_lyrics(&t2.id).await.unwrap();
        assert_eq!(l.plain.as_deref(), Some("la la"));
        assert_eq!(
            f.db.with(|db| db.get_lyrics(&t2.id))
                .unwrap()
                .plain
                .as_deref(),
            Some("la la")
        );
        assert_eq!(
            f.svc.get_lyrics("nao-existe").await.unwrap_err().code(),
            "not_found"
        );
    }

    // @test RF-12
    #[tokio::test]
    async fn artista_e_enriquecido_e_persistido() {
        let f = fx();
        let t = track(&f);
        let aid = t.artist_id.unwrap();
        *f.fake.artist.lock().unwrap() = Some(ArtistDetails {
            mbid: "art-1".into(),
            name: "Daft Punk".into(),
            country: Some("FR".into()),
            genres: vec!["electronic".into(), "house".into()],
            bio: None,
            image_url: None,
        });
        let info = f.svc.get_artist(&aid).await.unwrap();
        assert_eq!(info.mbid.as_deref(), Some("art-1"));
        assert_eq!(info.country.as_deref(), Some("FR"));
        assert_eq!(info.genres, vec!["electronic", "house"]);
        assert_eq!(info.track_count, 1);
        // persistido
        let stored = f.db.with(|db| db.get_artist_info(&aid)).unwrap();
        assert_eq!(stored.country.as_deref(), Some("FR"));
    }

    // @test RF-27
    #[tokio::test]
    async fn artista_recebe_foto_na_primeira_consulta_apos_obter_mbid() {
        let f = fx();
        let t = track(&f);
        let aid = t.artist_id.unwrap();
        *f.fake.artist.lock().unwrap() = Some(ArtistDetails {
            mbid: "art-1".into(),
            name: "Daft Punk".into(),
            country: Some("FR".into()),
            genres: vec!["electronic".into()],
            bio: None,
            image_url: None,
        });
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(2, 2)
            .write_to(&mut bytes, image::ImageFormat::Jpeg)
            .unwrap();
        *f.fake.photo.lock().unwrap() = Some(miupy_media::ArtistPhoto {
            bytes: bytes.into_inner(),
            extension: "jpg".into(),
            credit: "Autora — CC BY".into(),
        });
        let info = f.svc.get_artist(&aid).await.unwrap();
        assert_eq!(info.mbid.as_deref(), Some("art-1"));
        assert!(
            info.image_path
                .as_ref()
                .is_some_and(|p| std::path::Path::new(p).is_file())
        );
        assert_eq!(info.image_credit.as_deref(), Some("Autora — CC BY"));
    }

    // @test RF-12
    #[tokio::test]
    async fn artista_sem_rede_devolve_o_que_ha_no_banco() {
        let f = fx();
        let t = track(&f);
        let aid = t.artist_id.unwrap();
        *f.fake.fail_with.lock().unwrap() = Some(MediaError::Network("offline".into()));
        let info = f.svc.get_artist(&aid).await.unwrap();
        assert_eq!(info.name, "daft punk");
        assert!(info.mbid.is_none());
        assert_eq!(
            f.svc.get_artist("nao-existe").await.unwrap_err().code(),
            "not_found"
        );
    }

    // @test RF-27
    #[tokio::test]
    async fn falha_temporaria_da_foto_nao_bloqueia_por_cinco_minutos() {
        let f = fx();
        let t = track(&f);
        let aid = t.artist_id.unwrap();
        f.db.with(|db| {
            db.update_artist(
                &aid,
                &ArtistPatch {
                    mbid: Some(Some("art-1".into())),
                    country: Some(Some("FR".into())),
                    genres: Some(vec!["electronic".into()]),
                    ..Default::default()
                },
            )
        })
        .unwrap();
        *f.fake.fail_with.lock().unwrap() = Some(MediaError::Network("HTTP 503".into()));
        let info = f.svc.get_artist(&aid).await.unwrap();
        assert!(info.image_path.is_none());
        let attempts = f.svc.attempts.lock().unwrap();
        let age = attempts
            .get(&format!("artist-photo:{aid}"))
            .unwrap()
            .elapsed();
        assert!(age >= RETRY_AFTER - TEMPORARY_RETRY_AFTER);
    }

    // @test RF-27
    #[tokio::test]
    async fn erro_definitivo_da_foto_mantem_intervalo_longo() {
        let f = fx();
        let t = track(&f);
        let aid = t.artist_id.unwrap();
        f.db.with(|db| {
            db.update_artist(
                &aid,
                &ArtistPatch {
                    mbid: Some(Some("art-1".into())),
                    country: Some(Some("FR".into())),
                    genres: Some(vec!["electronic".into()]),
                    ..Default::default()
                },
            )
        })
        .unwrap();
        *f.fake.fail_with.lock().unwrap() = Some(MediaError::NotFound("artista".into()));
        f.svc.get_artist(&aid).await.unwrap();
        let attempts = f.svc.attempts.lock().unwrap();
        let age = attempts
            .get(&format!("artist-photo:{aid}"))
            .unwrap()
            .elapsed();
        assert!(age < TEMPORARY_RETRY_AFTER);
    }

    // @test RF-12
    #[tokio::test]
    async fn letra_nao_encontrada_mantem_intervalo_longo() {
        let f = fx();
        let t = track(&f);
        f.svc.get_lyrics(&t.id).await.unwrap();
        let attempts = f.svc.attempts.lock().unwrap();
        assert!(
            attempts.get(&format!("lyrics:{}", t.id)).unwrap().elapsed() < TEMPORARY_RETRY_AFTER
        );
    }
}
