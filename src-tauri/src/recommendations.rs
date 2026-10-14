//! Busca limitada de candidatas e ranking local para recomendações.

use std::sync::Arc;

use miupy_core::{
    Recommendation, RecommendationCandidate, RecommendationReasonKind, RecommendationSeed,
};
use miupy_media::{MediaError, MediaResolver, SearchResult};
use tokio::task::JoinSet;

use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::events::Events;
use crate::youtube::Youtube;

pub struct Recommendations {
    db: Db,
    resolver: Arc<dyn MediaResolver>,
    events: Arc<dyn Events>,
    youtube: Arc<Youtube>,
}

impl Recommendations {
    pub fn new(
        db: Db,
        resolver: Arc<dyn MediaResolver>,
        events: Arc<dyn Events>,
        youtube: Arc<Youtube>,
    ) -> Self {
        Self {
            db,
            resolver,
            events,
            youtube,
        }
    }

    // @req RF-19
    pub async fn list(
        &self,
        playlist_id: Option<String>,
        limit: Option<u32>,
    ) -> AppResult<Vec<Recommendation>> {
        self.db
            .run(move |db| Ok(db.list_recommendations(playlist_id.as_deref(), limit)?))
            .await
    }

    // @req RF-19
    pub async fn dismiss(&self, source_id: String) -> AppResult<()> {
        self.db
            .run(move |db| Ok(db.dismiss_recommendation(&source_id)?))
            .await?;
        self.emit_count().await;
        Ok(())
    }

    async fn emit_count(&self) {
        let count = self.list(None, None).await.map(|v| v.len()).unwrap_or(0);
        self.events.recommendations_changed(count);
    }

    // @req RF-19
    pub async fn refresh(&self, playlist_id: Option<String>) -> AppResult<Vec<Recommendation>> {
        // Nove chamadas no máximo; três em voo. Uma semente ruim não interrompe as demais.
        let pid = playlist_id.clone();
        let seeds = self
            .db
            .run(move |db| Ok(db.recommendation_seeds(pid.as_deref(), 3, 3, 3)?))
            .await?;
        let mut pending = seeds.into_iter();
        let mut tasks = JoinSet::new();
        let mut successes = 0;
        let mut network_errors = 0;
        let mut other_errors = 0;
        for _ in 0..3 {
            if let Some(seed) = pending.next() {
                spawn_seed(&mut tasks, self.resolver.clone(), seed);
            }
        }
        while let Some(outcome) = tasks.join_next().await {
            if let Some(seed) = pending.next() {
                spawn_seed(&mut tasks, self.resolver.clone(), seed);
            }
            match outcome {
                Ok(Ok((seed, hits))) => {
                    successes += 1;
                    self.youtube.cache_results(&hits);
                    for hit in hits.into_iter().filter(is_music_candidate) {
                        let (title, artist) = crate::youtube::display_fields(&hit);
                        let candidate = RecommendationCandidate {
                            source_id: hit.video_id,
                            title,
                            artist,
                            duration_ms: hit.duration_ms.unwrap_or(0) as i64,
                            thumbnail_url: hit.thumbnail_url,
                            genres: if seed.kind == RecommendationReasonKind::Genre {
                                vec![seed.label.clone()]
                            } else {
                                vec![]
                            },
                            seed_kind: seed.kind,
                            seed_label: seed.label.clone(),
                            seed_track_id: seed.track_id.clone(),
                        };
                        self.db
                            .run(move |db| Ok(db.upsert_recommendation_candidate(&candidate)?))
                            .await?;
                    }
                }
                Ok(Err(MediaError::Network(_))) => network_errors += 1,
                Ok(Err(e)) => {
                    other_errors += 1;
                    log::warn!("semente de recomendação falhou: {e}");
                }
                Err(e) => {
                    other_errors += 1;
                    log::warn!("tarefa de recomendação falhou: {e}");
                }
            }
        }
        if successes == 0 && network_errors > 0 && other_errors == 0 {
            return Err(AppError::from(MediaError::Network(
                "Sem conexão para atualizar as recomendações.".into(),
            )));
        }
        let list = self.list(playlist_id, None).await?;
        self.emit_count().await;
        Ok(list)
    }
}

fn spawn_seed(
    tasks: &mut JoinSet<Result<(RecommendationSeed, Vec<SearchResult>), MediaError>>,
    resolver: Arc<dyn MediaResolver>,
    seed: RecommendationSeed,
) {
    tasks.spawn(async move {
        let hits = match seed.kind {
            RecommendationReasonKind::SimilarTrack => {
                resolver
                    .related_tracks(seed.source_id.as_deref().unwrap_or(""), 8)
                    .await?
            }
            RecommendationReasonKind::Artist => {
                resolver.search(&format!("{} music", seed.label), 8).await?
            }
            RecommendationReasonKind::Genre => {
                resolver.search(&format!("{} music", seed.label), 8).await?
            }
        };
        Ok((seed, hits))
    });
}

/// Descarta lives sem duração e vídeos acima de 15 minutos, comuns em mixes e compilações.
fn is_music_candidate(hit: &SearchResult) -> bool {
    matches!(hit.duration_ms, Some(30_000..=900_000))
}
