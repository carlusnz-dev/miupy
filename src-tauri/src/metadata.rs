//! Enriquecimento de metadados (RF-12): MusicBrainz, Cover Art Archive e LRCLIB, via
//! `MetadataProvider`. Falha de rede nunca derruba a leitura local.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use miupy_core::{ArtistInfo, ArtistPatch, Lyrics, Track, TrackPatch};
use miupy_media::{MediaError, MediaResolver, MetadataProvider, TagUpdate, clean_title, tagger};

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

/// Fecha o lote. Parar depois da última faixa conta como concluído, não como "Parado em 3 de 3".
// @req RF-32
fn finish_batch(status: &mut MetadataRefreshStatus) {
    status.running = false;
    status.cancelled = status.stopping && status.done < status.total;
    status.stopping = false;
    status.current_track_id = None;
    status.finished_at = Some(now_ms());
}

// @req RF-32
/// Fecha o lote ao sair da tarefa, inclusive por pânico; no fim normal não faz nada.
struct BatchGuard(Arc<MetadataService>);

impl Drop for BatchGuard {
    fn drop(&mut self) {
        self.0.abort_batch();
    }
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

    // @req RF-32
    pub fn cancel_library_refresh(&self) {
        let mut status = self.batch_status.lock().unwrap_or_else(|e| e.into_inner());
        if !status.running || status.stopping {
            return;
        }
        status.stopping = true;
        let snapshot = status.clone();
        self.events.metadata_progress(&snapshot);
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
            let stopping = status.stopping;
            *status = MetadataRefreshStatus {
                running: true,
                stopping,
                total,
                started_at: Some(now_ms()),
                ..MetadataRefreshStatus::default()
            };
        });
        let this = self.clone();
        tauri::async_runtime::spawn(async move {
            // Se a tarefa morrer no meio (pânico), o lote não fica preso em `running`.
            let _guard = BatchGuard(this.clone());
            for (i, t) in tracks.into_iter().enumerate() {
                {
                    let mut status = this.batch_status.lock().unwrap_or_else(|e| e.into_inner());
                    if status.stopping {
                        break;
                    }
                    status.current_track_id = Some(t.id.clone());
                }
                let mut failed = false;
                if let Err(e) = this.refresh_track_inner(&t.id, false, true).await {
                    log::warn!("reprocessamento de {}: {e}", t.id);
                    failed = true;
                }
                this.update_batch_status(|status| {
                    status.done = i + 1;
                    status.failed += usize::from(failed);
                });
            }
            this.events.library_changed("metadata_batch_finished");
            this.update_batch_status(finish_batch);
        });
        Ok(total)
    }

    /// Encerra o lote em andamento (usado quando a tarefa do lote termina sem chegar ao fim).
    fn abort_batch(&self) {
        let running = self
            .batch_status
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .running;
        if running {
            self.update_batch_status(finish_batch);
        }
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
        self.refresh_track_inner(track_id, true, false).await
    }

    // @req RF-12 RF-32
    async fn refresh_track_inner(
        &self,
        track_id: &str,
        fetch_lyrics: bool,
        allow_inverted: bool,
    ) -> AppResult<Track> {
        let id = track_id.to_owned();
        let track = self.db.run(move |db| Ok(db.get_track(&id)?)).await?;
        let duration = u64::try_from(track.duration_ms).ok().filter(|d| *d > 0);

        let mut rec = self
            .provider
            .find_recording(&track.title, track.artist_name.as_deref(), duration)
            .await?;
        // @req RF-12: no lote, a ordem salva invertida exige confirmação forte do MusicBrainz.
        let mut suspected_inversion = false;
        let mut confirmed_inversion = false;
        let mut confirmed_video_title = None;
        let video = if let Some(source_id) = &track.source_id {
            self.resolver.video_metadata(source_id).await.ok()
        } else {
            None
        };
        if let Some(video) = &video
            && let Some(raw) = video.raw_title.as_deref()
        {
            let mut parsed = clean_title(raw, video.channel.as_deref().unwrap_or(""));
            // @req RF-12: o título bruto pode explicitar o artista completo mesmo quando
            // o canal só contém a primeira palavra; a confirmação ainda vem do MusicBrainz.
            if allow_inverted
                && let Some((left, right)) = raw.split_once(" - ")
                && track
                    .artist_name
                    .as_deref()
                    .is_some_and(|saved| saved.eq_ignore_ascii_case(left))
                && right
                    .to_lowercase()
                    .starts_with(&format!("{} ", track.title.to_lowercase()))
            {
                parsed.artist = Some(right.to_string());
                parsed.title = left.to_string();
            }
            if let Some(artist) = parsed.artist.as_deref() {
                let saved_artist_is_channel = video.channel.as_deref().is_some_and(|channel| {
                    track
                        .artist_name
                        .as_deref()
                        .is_some_and(|name| name.trim().eq_ignore_ascii_case(channel.trim()))
                });
                let artist_lower = artist.to_lowercase();
                let title_lower = track.title.to_lowercase();
                suspected_inversion = allow_inverted
                    && !saved_artist_is_channel
                    && track
                        .artist_name
                        .as_deref()
                        .is_some_and(|saved| saved.eq_ignore_ascii_case(&parsed.title))
                    && artist_lower
                        .strip_prefix(&title_lower)
                        .is_some_and(|tail| tail.is_empty() || tail.starts_with(' '));
                if saved_artist_is_channel || suspected_inversion {
                    let mut queries = vec![
                        (parsed.title.as_str(), artist),
                        (artist, parsed.title.as_str()),
                    ];
                    if suspected_inversion && let Some(saved_artist) = track.artist_name.as_deref()
                    {
                        queries.push((saved_artist, track.title.as_str()));
                    }
                    let mut seen = Vec::new();
                    for (title, artist) in queries {
                        if title.eq_ignore_ascii_case(&track.title)
                            && track
                                .artist_name
                                .as_deref()
                                .is_some_and(|saved| saved.eq_ignore_ascii_case(artist))
                        {
                            continue;
                        }
                        if seen.iter().any(|(seen_title, seen_artist): &(&str, &str)| {
                            seen_title.eq_ignore_ascii_case(title)
                                && seen_artist.eq_ignore_ascii_case(artist)
                        }) {
                            continue;
                        }
                        seen.push((title, artist));
                        let candidate = match self
                            .provider
                            .find_recording(title, Some(artist), duration)
                            .await
                        {
                            Ok(candidate) => candidate,
                            Err(error) => {
                                log::warn!(
                                    "consulta extra ao MusicBrainz para {}: {error}",
                                    track.id
                                );
                                continue;
                            }
                        };
                        if let Some(candidate) = candidate
                            && !candidate.title_only
                            && (!suspected_inversion
                                || (candidate.score >= 90
                                    && track.artist_name.as_deref().is_some_and(|saved| {
                                        candidate.title.eq_ignore_ascii_case(saved)
                                    })
                                    && {
                                        let candidate_artist = candidate.artist.to_lowercase();
                                        let saved_title = track.title.to_lowercase();
                                        candidate_artist == saved_title
                                            || (saved_title.split_whitespace().count() >= 2
                                                && candidate_artist
                                                    .starts_with(&format!("{saved_title} ")))
                                            || candidate_artist == artist.to_lowercase()
                                            || video.channel.as_deref().is_some_and(|channel| {
                                                channel.to_lowercase().ends_with(" tv")
                                                    && candidate_artist
                                                        .starts_with(&format!("{} ", saved_title))
                                            })
                                    }))
                            && rec.as_ref().is_none_or(|current| {
                                current.title_only
                                    || candidate.score > current.score
                                    || (suspected_inversion && candidate.score == current.score)
                            })
                        {
                            confirmed_inversion = suspected_inversion;
                            if suspected_inversion {
                                confirmed_video_title = Some(parsed.title.clone());
                            }
                            rec = Some(candidate);
                        }
                    }
                }
            }
        }
        if suspected_inversion && !confirmed_inversion {
            return Ok(track);
        }
        let Some(rec) = rec else {
            // @req RF-20: sem correspondência no MusicBrainz, usa apenas os campos
            // musicais do vídeo; suas categorias nunca viram gênero.
            let Some(video) = video else {
                return Ok(track);
            };
            let patch = TrackPatch {
                title: video.track,
                artist_name: video.artist.map(Some),
                album: track
                    .album
                    .is_none()
                    .then_some(video.album)
                    .flatten()
                    .map(Some),
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

        // Busca só por título não confirma o lançamento nem sua capa.
        let mut mbids = if rec.title_only {
            Vec::new()
        } else {
            rec.release_mbids.clone()
        };
        if mbids.is_empty() && !rec.title_only {
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
        let lyrics = if fetch_lyrics && !rec.title_only {
            match self
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
            }
        } else {
            None
        };

        // @req RF-20: MusicBrainz é a fonte principal; o vídeo só complementa campos ausentes.
        // @req RF-12: só corrige canal salvo como artista com uma candidata e duração conhecida.
        let correct_artist = rec.title_only
            && rec.title_only_artist_unambiguous
            && duration.is_some()
            && video
                .as_ref()
                .and_then(|v| v.channel.as_deref())
                .is_some_and(|channel| {
                    track
                        .artist_name
                        .as_deref()
                        .is_some_and(|name| name.trim().eq_ignore_ascii_case(channel.trim()))
                });
        let artist_name = if confirmed_inversion {
            rec.artist.replace('’', "'")
        } else {
            rec.artist.clone()
        };
        let patch = TrackPatch {
            title: (!rec.title_only)
                .then(|| confirmed_video_title.unwrap_or_else(|| rec.title.clone()))
                .filter(|t| !t.trim().is_empty())
                .or_else(|| {
                    (!rec.title_only)
                        .then(|| video.as_ref().and_then(|v| v.track.clone()))
                        .flatten()
                }),
            artist_name: (correct_artist || !rec.title_only)
                .then(|| artist_name.clone())
                .filter(|a| !a.trim().is_empty())
                .or_else(|| {
                    (!rec.title_only)
                        .then(|| video.as_ref().and_then(|v| v.artist.clone()))
                        .flatten()
                })
                .filter(|a| !a.trim().is_empty())
                .map(Some),
            album: (track.album.is_none() && !rec.title_only)
                .then(|| {
                    rec.album
                        .clone()
                        .or_else(|| video.as_ref().and_then(|v| v.album.clone()))
                })
                .flatten()
                .map(Some),
            duration_ms: if track.duration_ms <= 0 && !rec.title_only {
                rec.duration_ms.and_then(|d| i64::try_from(d).ok())
            } else {
                None
            },
            cover_path: cover
                .as_ref()
                .map(|p| Some(p.to_string_lossy().into_owned())),
            genres: (!rec.genres.is_empty()).then(|| rec.genres.clone()),
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
                if !rec.title_only
                    && let Some(aid) = &updated.artist_id
                {
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

    // @test RF-32
    #[test]
    fn parar_na_ultima_faixa_conta_como_concluido() {
        let mut status = MetadataRefreshStatus {
            running: true,
            stopping: true,
            done: 3,
            total: 3,
            ..Default::default()
        };
        finish_batch(&mut status);
        assert!(!status.cancelled && !status.running && !status.stopping);
        status = MetadataRefreshStatus {
            stopping: true,
            done: 1,
            total: 3,
            ..Default::default()
        };
        finish_batch(&mut status);
        assert!(status.cancelled);
    }
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
            title_only: false,
            title_only_artist_unambiguous: false,
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

    // @test RF-12
    #[tokio::test]
    async fn reprocessamento_consulta_duas_ordens_so_para_artista_igual_ao_canal() {
        for (saved_artist, expected_calls) in [("Canal", 3), ("Artista salvo", 1)] {
            let f = fx();
            let t =
                f.db.with(|db| {
                    db.upsert_track(&NewTrack::youtube(
                        "vid1",
                        "Título antigo",
                        Some(saved_artist.into()),
                        200_000,
                        None,
                    ))
                })
                .unwrap()
                .track;
            let mut matching = rec();
            matching.release_mbids.clear();
            matching.release_mbid = None;
            *f.fake.recording.lock().unwrap() = Some(matching);
            *f.resolver.video.lock().unwrap() = Some(miupy_media::VideoMetadata {
                channel: Some("Canal".into()),
                raw_title: Some("Matuê - Kenny G".into()),
                ..Default::default()
            });
            f.svc.refresh_track_inner(&t.id, false, true).await.unwrap();
            assert_eq!(*f.fake.network_calls.lock().unwrap(), expected_calls);
        }
    }

    // @test RF-12
    #[tokio::test]
    async fn erro_na_consulta_extra_preserva_primeira_resposta() {
        let f = fx();
        let track =
            f.db.with(|db| {
                db.upsert_track(&NewTrack::youtube(
                    "vid1",
                    "Título antigo",
                    Some("Canal".into()),
                    200_000,
                    None,
                ))
            })
            .unwrap()
            .track;
        let mut matching = rec();
        matching.release_mbids.clear();
        matching.release_mbid = None;
        *f.fake.recording.lock().unwrap() = Some(matching);
        *f.fake.fail_on_call.lock().unwrap() = Some(2);
        *f.fake.fail_with.lock().unwrap() = Some(MediaError::Network("sem rede".into()));
        *f.resolver.video.lock().unwrap() = Some(miupy_media::VideoMetadata {
            channel: Some("Canal".into()),
            raw_title: Some("Matuê - Kenny G".into()),
            ..Default::default()
        });
        let saved = f
            .svc
            .refresh_track_inner(&track.id, false, true)
            .await
            .unwrap();
        assert_eq!(saved.title, "Get Lucky");
        assert_eq!(saved.artist_name.as_deref(), Some("Daft Punk"));
    }

    // @test RF-12
    #[tokio::test]
    async fn reprocessamento_corrige_inversao_so_com_gravacao_forte() {
        for (channel, score, title_only) in [
            ("Racionais", Some(100), false),
            ("Racionais MC's", Some(100), false),
            ("Racionais", Some(89), false),
            ("Racionais", Some(100), true),
            ("Racionais", None, false),
        ] {
            let f = fx();
            let t =
                f.db.with(|db| {
                    db.upsert_track(&NewTrack::youtube(
                        "vid1",
                        "Racionais",
                        Some("Negro Drama".into()),
                        200_000,
                        None,
                    ))
                })
                .unwrap()
                .track;
            *f.resolver.video.lock().unwrap() = Some(miupy_media::VideoMetadata {
                channel: Some(channel.into()),
                raw_title: Some("NEGRO DRAMA - RACIONAIS MC'S".into()),
                track: Some("Negro Drama".into()),
                artist: Some("Racionais MC's".into()),
                ..Default::default()
            });
            if let Some(score) = score {
                let mut mb = rec();
                mb.title = "NEGRO DRAMA".into();
                mb.artist = "RACIONAIS MC'S".into();
                mb.score = score;
                mb.title_only = title_only;
                mb.release_mbids.clear();
                mb.release_mbid = None;
                *f.fake.recording.lock().unwrap() = Some(mb);
            }
            let saved = f.svc.refresh_track_inner(&t.id, false, true).await.unwrap();
            if score == Some(100) && !title_only {
                assert_eq!(saved.title, "NEGRO DRAMA");
                assert_eq!(saved.artist_name.as_deref(), Some("RACIONAIS MC'S"));
            } else {
                assert_eq!(saved.title, t.title);
                assert_eq!(saved.artist_name, t.artist_name);
            }
        }
    }

    // @test RF-12
    #[tokio::test]
    async fn reprocessamento_preserva_grafia_do_video_apos_confirmacao() {
        let f = fx();
        let track =
            f.db.with(|db| {
                db.upsert_track(&NewTrack::youtube(
                    "vid1",
                    "Racionais",
                    Some("Negro Drama".into()),
                    200_000,
                    None,
                ))
            })
            .unwrap()
            .track;
        *f.resolver.video.lock().unwrap() = Some(miupy_media::VideoMetadata {
            channel: Some("Racionais TV".into()),
            raw_title: Some("Negro Drama - Nada Como Um Dia Após O Outro Dia (Chora Agora)".into()),
            ..Default::default()
        });
        let mut mb = rec();
        mb.title = "Negro drama".into();
        mb.artist = "Racionais MC’s".into();
        mb.score = 90;
        mb.release_mbids.clear();
        mb.release_mbid = None;
        *f.fake.recording.lock().unwrap() = Some(mb);

        let saved = f
            .svc
            .refresh_track_inner(&track.id, false, true)
            .await
            .unwrap();
        assert_eq!(saved.title, "Negro Drama");
        assert_eq!(saved.artist_name.as_deref(), Some("Racionais MC's"));
    }

    // @test RF-12
    #[tokio::test]
    async fn artista_e_titulo_corretos_permanecem_corretos() {
        let f = fx();
        let t =
            f.db.with(|db| {
                db.upsert_track(&NewTrack::youtube(
                    "vid1",
                    "Negro Drama",
                    Some("Racionais MC's".into()),
                    200_000,
                    None,
                ))
            })
            .unwrap()
            .track;
        *f.resolver.video.lock().unwrap() = Some(miupy_media::VideoMetadata {
            channel: Some("Racionais".into()),
            raw_title: Some("NEGRO DRAMA - RACIONAIS MC'S".into()),
            ..Default::default()
        });
        let mut mb = rec();
        mb.title = "Negro Drama".into();
        mb.artist = "Racionais MC's".into();
        mb.release_mbids.clear();
        mb.release_mbid = None;
        *f.fake.recording.lock().unwrap() = Some(mb);
        let saved = f.svc.refresh_track_inner(&t.id, false, true).await.unwrap();
        assert_eq!(saved.title, t.title);
        assert_eq!(saved.artist_name, t.artist_name);
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
    async fn parar_termina_faixa_atual_e_permite_novo_lote() {
        let f = fx();
        track(&f);
        for n in 2..=3 {
            f.db.with(|db| {
                db.upsert_track(&NewTrack::youtube(
                    format!("vid{n}"),
                    "Outra",
                    None,
                    1000,
                    None,
                ))
            })
            .unwrap();
        }
        *f.fake.network_delay.lock().unwrap() = Some(Duration::from_millis(100));
        let svc = Arc::new(f.svc);
        svc.cancel_library_refresh();
        assert_eq!(svc.refresh_status(), MetadataRefreshStatus::default());
        assert_eq!(svc.enqueue_library_refresh(false).unwrap(), 3);
        tokio::time::timeout(Duration::from_secs(3), async {
            while *f.fake.network_calls.lock().unwrap() == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        svc.cancel_library_refresh();
        assert!(svc.refresh_status().stopping);
        tokio::time::timeout(Duration::from_secs(3), async {
            while svc.refresh_status().running {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let end = svc.refresh_status();
        assert_eq!((end.done, end.total, end.failed), (1, 3, 0));
        assert!(end.cancelled);
        assert!(!end.stopping);
        svc.cancel_library_refresh();
        assert_eq!(svc.refresh_status(), end);
        assert_eq!(svc.enqueue_library_refresh(false).unwrap(), 3);
        tokio::time::timeout(Duration::from_secs(3), async {
            while svc.refresh_status().running {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let next = svc.refresh_status();
        assert_eq!(next.done, 3);
        assert!(!next.cancelled);
    }

    // @test RF-32 RF-12
    #[tokio::test]
    async fn lote_avanca_sem_consultar_letras() {
        let f = fx();
        track(&f);
        let mut recording = rec();
        recording.release_mbids.clear();
        recording.release_mbid = None;
        *f.fake.recording.lock().unwrap() = Some(recording);
        *f.fake.lyrics.lock().unwrap() = Some(LyricsData {
            plain: Some("letra".into()),
            synced: None,
            instrumental: false,
        });
        let svc = Arc::new(f.svc);
        svc.enqueue_library_refresh(false).unwrap();
        tokio::time::timeout(Duration::from_secs(3), async {
            while svc.refresh_status().running {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(svc.refresh_status().done, 1);
        assert_eq!(*f.fake.network_calls.lock().unwrap(), 1);
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

    // @test RF-32
    #[test]
    fn guarda_do_lote_encerra_lote_interrompido_e_libera_novo_pedido() {
        let f = fx();
        let svc = Arc::new(f.svc);
        svc.batch_status.lock().unwrap().running = true;
        svc.batch_status.lock().unwrap().current_track_id = Some("t".into());
        // A tarefa do lote "morre" (pânico) sem chegar ao fim: só a guarda é solta.
        drop(BatchGuard(svc.clone()));
        let status = svc.refresh_status();
        assert!(!status.running);
        assert_eq!(status.current_track_id, None);
        assert!(status.finished_at.is_some());
        // Lote já encerrado: a guarda não mexe em nada.
        drop(BatchGuard(svc.clone()));
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
            channel: None,
            raw_title: None,
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
            channel: None,
            raw_title: None,
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

    // @test RF-12 RF-20
    #[tokio::test]
    async fn busca_so_por_titulo_preserva_campos_e_corrige_canal_com_seguranca() {
        for (channel, raw, unique, duration_ms, expected_artist) in [
            ("Canal Errado", "Get Lucky", true, 369_000, "Daft Punk"),
            ("Canal Errado", "Get Lucky", true, 0, "Canal Errado"),
            ("Canal Errado", "Get Lucky", false, 369_000, "Canal Errado"),
            ("Outro canal", "Get Lucky", true, 369_000, "Canal Errado"),
            (
                "Outro canal",
                "Canal Errado - Get Lucky",
                true,
                369_000,
                "Canal Errado",
            ),
        ] {
            let f = fx();
            let t =
                f.db.with(|db| {
                    db.upsert_track(&NewTrack::youtube(
                        "vid1",
                        "get lucky",
                        Some("Canal Errado".into()),
                        duration_ms,
                        None,
                    ))
                })
                .unwrap()
                .track;
            let original_artist_id = t.artist_id.clone();
            let mut mb = rec();
            mb.title_only = true;
            mb.title_only_artist_unambiguous = unique;
            mb.genres = vec!["disco".into()];
            mb.release_year = Some(2013);
            mb.isrc = Some("USABC1300001".into());
            *f.fake.recording.lock().unwrap() = Some(mb);
            *f.resolver.video.lock().unwrap() = Some(miupy_media::VideoMetadata {
                channel: Some(channel.into()),
                raw_title: Some(raw.into()),
                ..Default::default()
            });
            let saved = f.svc.refresh_track(&t.id).await.unwrap();
            assert_eq!(saved.title, "get lucky");
            assert_eq!(saved.artist_name.as_deref(), Some(expected_artist));
            assert_eq!(saved.album, None);
            assert_eq!(saved.cover_path, None);
            assert_eq!(saved.genres, ["disco"]);
            assert_eq!(saved.release_year, Some(2013));
            assert_eq!(
                f.db.with(|db| db.get_artist_info(original_artist_id.as_ref().unwrap()))
                    .unwrap()
                    .name,
                "Canal Errado"
            );
            if unique && duration_ms > 0 && channel == "Canal Errado" {
                assert_ne!(saved.artist_id, original_artist_id);
            }
        }
    }

    // @test RF-12 RF-20
    #[tokio::test]
    async fn album_existente_e_preservado_nos_dois_caminhos() {
        for title_only in [false, true] {
            let f = fx();
            let t = track(&f);
            f.db.with(|db| {
                db.update_track(
                    &t.id,
                    &TrackPatch {
                        album: Some(Some("Álbum escolhido".into())),
                        ..Default::default()
                    },
                )
            })
            .unwrap();
            let mut mb = rec();
            mb.title_only = title_only;
            *f.fake.recording.lock().unwrap() = Some(mb);
            let saved = f.svc.refresh_track(&t.id).await.unwrap();
            assert_eq!(saved.album.as_deref(), Some("Álbum escolhido"));
        }
        let f = fx();
        let t = track(&f);
        f.db.with(|db| {
            db.update_track(
                &t.id,
                &TrackPatch {
                    album: Some(Some("Álbum escolhido".into())),
                    ..Default::default()
                },
            )
        })
        .unwrap();
        *f.resolver.video.lock().unwrap() = Some(miupy_media::VideoMetadata {
            album: Some("Álbum do vídeo".into()),
            ..Default::default()
        });
        assert_eq!(
            f.svc.refresh_track(&t.id).await.unwrap().album.as_deref(),
            Some("Álbum escolhido")
        );
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
