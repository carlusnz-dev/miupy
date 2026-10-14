//! Busca no YouTube e adição à biblioteca (RF-09).
//!
//! O contrato manda só `sourceIds` em `youtube_add_to_library`, então os resultados da última
//! busca ficam num cache em memória, de onde saem título, artista, duração e miniatura.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use miupy_core::{NewTrack, Source, Track, TrackQuery};
use miupy_media::{MediaResolver, clean_title};

use crate::db::Db;
use crate::dto::SearchResult;
use crate::error::{AppError, AppResult};
use crate::events::Events;

const DEFAULT_LIMIT: usize = 20;
const MAX_LIMIT: usize = 50;
const CACHE_LIMIT: usize = 400;

pub struct Youtube {
    db: Db,
    resolver: Arc<dyn MediaResolver>,
    events: Arc<dyn Events>,
    cache: Mutex<HashMap<String, miupy_media::SearchResult>>,
}

/// Título e artista exibidos (e gravados) para um vídeo: título limpo de ruído de vídeo, artista
/// do título ou, na falta, do canal.
fn display_fields(r: &miupy_media::SearchResult) -> (String, String) {
    let parsed = clean_title(&r.title, &r.channel);
    let title = if parsed.title.trim().is_empty() {
        r.title.clone()
    } else {
        parsed.title
    };
    let artist = parsed.artist.unwrap_or_else(|| r.channel.clone());
    (title, artist)
}

fn to_new_track(r: &miupy_media::SearchResult) -> NewTrack {
    let (title, artist) = display_fields(r);
    let artist = (!artist.trim().is_empty()).then_some(artist);
    NewTrack::youtube(
        r.video_id.clone(),
        title,
        artist,
        r.duration_ms.map_or(0, |d| i64::try_from(d).unwrap_or(0)),
        r.thumbnail_url.clone(),
    )
}

impl Youtube {
    pub fn new(db: Db, resolver: Arc<dyn MediaResolver>, events: Arc<dyn Events>) -> Self {
        Youtube {
            db,
            resolver,
            events,
            cache: Mutex::new(HashMap::new()),
        }
    }

    fn cache(&self) -> std::sync::MutexGuard<'_, HashMap<String, miupy_media::SearchResult>> {
        self.cache.lock().unwrap_or_else(|e| e.into_inner())
    }

    // @req RF-09
    pub async fn search(&self, query: &str, limit: Option<u32>) -> AppResult<Vec<SearchResult>> {
        let query = query.trim();
        if query.is_empty() {
            return Err(AppError::invalid("A busca está vazia."));
        }
        let limit = limit
            .map_or(DEFAULT_LIMIT, |l| l as usize)
            .clamp(1, MAX_LIMIT);
        let raw = self.resolver.search(query, limit).await?;

        let ids: Vec<String> = raw.iter().map(|r| r.video_id.clone()).collect();
        let known = self
            .db
            .run(move |db| Ok(db.known_source_ids(Source::Youtube, &ids)?))
            .await?;
        {
            let mut cache = self.cache();
            if cache.len() + raw.len() > CACHE_LIMIT {
                cache.clear();
            }
            for r in &raw {
                cache.insert(r.video_id.clone(), r.clone());
            }
        }
        let out = raw
            .iter()
            .map(|r| {
                let (title, artist) = display_fields(r);
                SearchResult {
                    source_id: r.video_id.clone(),
                    title,
                    artist,
                    duration_ms: r.duration_ms.unwrap_or(0),
                    thumbnail_url: r.thumbnail_url.clone(),
                    in_library: known.contains(&r.video_id),
                }
            })
            .collect();
        self.db.with(|db| {
            let _ = db.log_action(
                "youtube_search",
                None,
                None,
                Some(&serde_json::json!({ "query": query })),
            );
        });
        Ok(out)
    }

    // @req RF-09
    /// Cria as faixas (sem arquivo) e, se pedido, as acrescenta à playlist. Faixas que já
    /// estavam na biblioteca são devolvidas sem duplicar.
    pub async fn add_to_library(
        &self,
        source_ids: Vec<String>,
        playlist_id: Option<String>,
    ) -> AppResult<Vec<Track>> {
        if source_ids.is_empty() {
            return Ok(Vec::new());
        }
        let cached: Vec<Option<miupy_media::SearchResult>> = {
            let cache = self.cache();
            source_ids.iter().map(|id| cache.get(id).cloned()).collect()
        };
        let tracks = self
            .db
            .run(move |db| {
                if let Some(pid) = &playlist_id {
                    db.get_playlist(pid)?;
                }
                // Resolve tudo antes de gravar qualquer coisa.
                let mut library: Option<Vec<Track>> = None;
                let mut plan: Vec<Result<NewTrack, Track>> = Vec::new();
                for (sid, hit) in source_ids.iter().zip(cached) {
                    match hit {
                        Some(r) => plan.push(Ok(to_new_track(&r))),
                        None => {
                            if library.is_none() {
                                library = Some(db.list_tracks(&TrackQuery::default())?);
                            }
                            let all = library.as_deref().unwrap_or_default();
                            match all.iter().find(|t| t.source_id.as_deref() == Some(sid)) {
                                Some(t) => plan.push(Err(t.clone())),
                                None => {
                                    return Err(AppError::not_found(
                                        "Resultado da busca expirado. Busque de novo.",
                                    ));
                                }
                            }
                        }
                    }
                }
                let mut tracks = Vec::new();
                for step in plan {
                    match step {
                        Ok(new) => tracks.push(db.upsert_track(&new)?.track),
                        Err(existing) => tracks.push(existing),
                    }
                }
                if let Some(pid) = &playlist_id {
                    let ids: Vec<String> = tracks.iter().map(|t| t.id.clone()).collect();
                    db.add_tracks_to_playlist(pid, &ids)?;
                }
                Ok(tracks)
            })
            .await?;
        self.events.library_changed("tracks_added");
        Ok(tracks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::downloads::testing::{FakeResolver, sr};
    use crate::events::testing::RecordingEvents;
    use miupy_core::Database;
    use miupy_media::MediaError;

    fn svc() -> (Youtube, Arc<FakeResolver>, Db, Arc<RecordingEvents>) {
        let db = Db::new(Database::open_in_memory().unwrap());
        let resolver = Arc::new(FakeResolver::default());
        resolver.results.lock().unwrap().extend([
            sr("id1", "Daft Punk - Get Lucky (Official Video)", "Daft Punk"),
            sr("id2", "Get Lucky (Lyrics)", "Outro Canal"),
            sr("id3", "Get Lucky live", "Daft Punk - Topic"),
        ]);
        let events = Arc::new(RecordingEvents::default());
        (
            Youtube::new(db.clone(), resolver.clone(), events.clone()),
            resolver,
            db,
            events,
        )
    }

    // @test RF-09
    #[tokio::test]
    async fn busca_separa_artista_e_titulo() {
        let (y, _, _, _) = svc();
        let r = y.search("get lucky", None).await.unwrap();
        assert_eq!(r.len(), 3);
        assert_eq!(r[0].source_id, "id1");
        assert_eq!(r[0].title, "Get Lucky");
        assert_eq!(r[0].artist, "Daft Punk");
        assert_eq!(r[0].duration_ms, 200_000);
        assert!(!r[0].in_library);
        // sem artista no título, vale o canal
        assert_eq!(r[1].artist, "Outro Canal");
        assert_eq!(r[2].artist, "Daft Punk");
    }

    // @test RF-09
    #[tokio::test]
    async fn busca_vazia_e_recusada_e_erro_do_resolver_passa_com_codigo() {
        let (y, resolver, _, _) = svc();
        assert_eq!(
            y.search("   ", None).await.unwrap_err().code(),
            "invalid_input"
        );
        *resolver.fail_with.lock().unwrap() =
            Some(MediaError::Unavailable("instale o yt-dlp".into()));
        let err = y.search("x", None).await.unwrap_err();
        assert_eq!(err.code(), "unavailable");
        assert!(err.to_string().contains("instale o yt-dlp"));
    }

    // @test RF-09
    #[tokio::test]
    async fn adicionar_cria_faixas_marca_in_library_e_nao_duplica() {
        let (y, _, db, events) = svc();
        y.search("get lucky", None).await.unwrap();
        let t = y
            .add_to_library(vec!["id1".into(), "id2".into()], None)
            .await
            .unwrap();
        assert_eq!(t.len(), 2);
        assert_eq!(t[0].title, "Get Lucky");
        assert_eq!(t[0].artist_name.as_deref(), Some("Daft Punk"));
        assert_eq!(t[0].file_path, None);
        assert_eq!(t[0].duration_ms, 200_000);
        assert!(
            events
                .library
                .lock()
                .unwrap()
                .contains(&"tracks_added".to_string())
        );

        let again = y.add_to_library(vec!["id1".into()], None).await.unwrap();
        assert_eq!(again[0].id, t[0].id);
        assert_eq!(
            db.with(|db| db.list_tracks(&TrackQuery::default()))
                .unwrap()
                .len(),
            2
        );

        let r = y.search("get lucky", None).await.unwrap();
        assert!(r[0].in_library && r[1].in_library && !r[2].in_library);
    }

    // @test RF-09 RF-06
    #[tokio::test]
    async fn adicionar_com_playlist_acrescenta_na_ordem() {
        let (y, _, db, _) = svc();
        let pl = db.with(|db| db.create_playlist("Favs", None)).unwrap();
        y.search("get lucky", None).await.unwrap();
        let t = y
            .add_to_library(vec!["id2".into(), "id1".into()], Some(pl.id.clone()))
            .await
            .unwrap();
        let in_pl = db
            .with(|db| db.playlist_tracks(&pl.id, &TrackQuery::default()))
            .unwrap();
        assert_eq!(
            in_pl.iter().map(|x| &x.id).collect::<Vec<_>>(),
            vec![&t[0].id, &t[1].id]
        );
    }

    // @test RF-09
    #[tokio::test]
    async fn id_desconhecido_nao_grava_nada() {
        let (y, _, db, _) = svc();
        y.search("get lucky", None).await.unwrap();
        let err = y
            .add_to_library(vec!["id1".into(), "naoexiste".into()], None)
            .await
            .unwrap_err();
        assert_eq!(err.code(), "not_found");
        assert!(
            db.with(|db| db.list_tracks(&TrackQuery::default()))
                .unwrap()
                .is_empty()
        );
        // playlist inexistente também falha antes de gravar
        let err = y
            .add_to_library(vec!["id1".into()], Some("nada".into()))
            .await
            .unwrap_err();
        assert_eq!(err.code(), "not_found");
    }

    // @test RF-09
    #[tokio::test]
    async fn faixa_ja_na_biblioteca_nao_depende_do_cache() {
        let (y, _, _, _) = svc();
        y.search("get lucky", None).await.unwrap();
        let first = y.add_to_library(vec!["id1".into()], None).await.unwrap();
        y.cache().clear();
        let again = y.add_to_library(vec!["id1".into()], None).await.unwrap();
        assert_eq!(first[0].id, again[0].id);
    }
}
