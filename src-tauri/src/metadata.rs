//! Enriquecimento de metadados (RF-12): MusicBrainz, Cover Art Archive e LRCLIB, via
//! `MetadataProvider`. Falha de rede nunca derruba a leitura local.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use miupy_core::{ArtistInfo, ArtistPatch, Lyrics, Track, TrackPatch};
use miupy_media::{MediaError, MetadataProvider, TagUpdate, tagger};

use crate::db::Db;
use crate::error::AppResult;
use crate::events::Events;
use crate::paths::Dirs;

/// Limite de espera por uma leitura enriquecida (artista, letra): offline não pode travar a tela.
const READ_TIMEOUT: Duration = Duration::from_secs(12);
/// Depois de uma tentativa (com ou sem sucesso), não repete a mesma consulta por este tempo.
const RETRY_AFTER: Duration = Duration::from_secs(300);

pub struct MetadataService {
    db: Db,
    provider: Arc<dyn MetadataProvider>,
    dirs: Dirs,
    events: Arc<dyn Events>,
    attempts: Mutex<HashMap<String, Instant>>,
}

impl MetadataService {
    pub fn new(
        db: Db,
        provider: Arc<dyn MetadataProvider>,
        dirs: Dirs,
        events: Arc<dyn Events>,
    ) -> Self {
        MetadataService {
            db,
            provider,
            dirs,
            events,
            attempts: Mutex::new(HashMap::new()),
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

    // @req RF-12
    /// Procura a gravação no MusicBrainz e atualiza título, artista, álbum, capa e letra.
    /// Aqui a rede é o objetivo: erro de rede volta ao usuário. Sem correspondência, devolve a
    /// faixa como estava.
    pub async fn refresh_track(&self, track_id: &str) -> AppResult<Track> {
        let id = track_id.to_owned();
        let track = self.db.run(move |db| Ok(db.get_track(&id)?)).await?;
        let duration = u64::try_from(track.duration_ms).ok().filter(|d| *d > 0);

        let Some(rec) = self
            .provider
            .find_recording(&track.title, track.artist_name.as_deref(), duration)
            .await?
        else {
            return Ok(track);
        };

        // Capa: tenta os releases candidatos em ordem; o primeiro que tiver capa vale.
        let mut mbids = rec.release_mbids.clone();
        if mbids.is_empty() {
            mbids.extend(rec.release_mbid.clone());
        }
        let mut cover = None;
        for mbid in mbids {
            match self
                .provider
                .download_cover(&mbid, &self.dirs.covers, &track.id)
                .await
            {
                Ok(path) => {
                    cover = Some(path);
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

        let patch = TrackPatch {
            title: Some(rec.title.clone()).filter(|t| !t.trim().is_empty()),
            artist_name: Some(rec.artist.clone())
                .filter(|a| !a.trim().is_empty())
                .map(Some),
            album: rec.album.clone().map(Some),
            duration_ms: if track.duration_ms <= 0 {
                rec.duration_ms.and_then(|d| i64::try_from(d).ok())
            } else {
                None
            },
            cover_path: cover
                .as_ref()
                .map(|p| Some(p.to_string_lossy().into_owned())),
            // gêneros, ano, MBID e ISRC passam a ser preenchidos pela casca na segunda onda
            ..Default::default()
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
            cover: cover.and_then(|p| std::fs::read(p).ok()),
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
        let info = self.db.run(move |db| Ok(db.get_artist_info(&id)?)).await?;
        let incomplete = info.mbid.is_none() || (info.country.is_none() && info.genres.is_empty());
        if !incomplete || !self.should_try(format!("artist:{}", info.id)) {
            return Ok(info);
        }
        match tokio::time::timeout(READ_TIMEOUT, self.enrich_artist(&info)).await {
            Ok(Ok(Some(updated))) => Ok(updated),
            Ok(Ok(None)) => Ok(info),
            Ok(Err(e)) => {
                log::info!("sem enriquecimento para o artista {}: {e}", info.name);
                Ok(info)
            }
            Err(_) => {
                log::info!("tempo esgotado ao enriquecer o artista {}", info.name);
                Ok(info)
            }
        }
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
                    Ok(p) => Some(p.to_string_lossy().into_owned()),
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
                return Ok(stored);
            }
            Err(_) => {
                log::info!("tempo esgotado buscando a letra de {}", track.id);
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
        let events = Arc::new(RecordingEvents::default());
        Fx {
            svc: MetadataService::new(db.clone(), fake.clone(), dirs.clone(), events.clone()),
            fake,
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
            release_mbids: vec!["rel-1".into(), "rel-2".into(), "rel-3".into()],
            duration_ms: Some(369_000),
            score: 100,
        }
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
        // nova tentativa dentro do intervalo não bate na rede de novo
        let calls = *f.fake.network_calls.lock().unwrap();
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
}
