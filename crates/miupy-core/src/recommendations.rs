//! Recomendação local. A rede e a busca de vídeos pertencem à casca; este módulo só lê e
//! grava SQLite e pontua candidatos com aritmética simples.

use std::collections::{HashMap, HashSet};

use rusqlite::{OptionalExtension, params};

use crate::db::Database;
use crate::error::{Error, Result};
use crate::model::{
    Recommendation, RecommendationCandidate, RecommendationReason, RecommendationReasonKind,
    RecommendationSeed, TasteArtist, TasteGenre, TasteProfile,
};
use crate::tracks::normalized_genres;

// @req RF-19
pub const RECENCY_HALF_LIFE_DAYS: f64 = 90.0;
pub const LISTEN_WEIGHT: f64 = 0.45;
pub const COMPLETE_WEIGHT: f64 = 0.80;
pub const SKIP_BASE_PENALTY: f64 = 1.10;
pub const SKIP_LATE_DISCOUNT: f64 = 0.70;
pub const FAVORITE_BONUS: f64 = 0.65;
pub const REPEAT_BONUS: f64 = 0.15;
const DAY_MS: f64 = 86_400_000.0;
const MAX_PER_ARTIST: usize = 3;

#[derive(Debug)]
struct ScoredTrack {
    id: String,
    title: String,
    source_id: Option<String>,
    artist_id: Option<String>,
    artist_name: Option<String>,
    genres: Vec<String>,
    score: f64,
}

#[derive(Default)]
struct TasteData {
    tracks: Vec<ScoredTrack>,
    artists: HashMap<String, (String, f64)>,
    genres: HashMap<String, f64>,
    total_listened_ms: i64,
    skipped: i64,
    ended: i64,
}

impl Database {
    fn taste_data(&self, playlist_id: Option<&str>) -> Result<TasteData> {
        if let Some(id) = playlist_id {
            self.get_playlist(id)?;
        }
        let mut data = TasteData::default();
        let now = self.now_ms();
        let mut stmt = self.conn.prepare(
            "SELECT t.id, t.title, t.source_id, t.artist_id, a.name, t.genres,
                    f.track_id IS NOT NULL, t.duration_ms,
                    e.started_at, e.listened_ms, e.completed, e.skipped
             FROM tracks t
             LEFT JOIN artists a ON a.id = t.artist_id
             LEFT JOIN favorites f ON f.track_id = t.id
             JOIN play_events e ON e.track_id = t.id AND e.ended_at IS NOT NULL
             WHERE (?1 IS NULL OR EXISTS (
                 SELECT 1 FROM playlist_tracks pt WHERE pt.playlist_id = ?1 AND pt.track_id = t.id))
             ORDER BY t.id, e.started_at",
        )?;
        let mut rows = stmt.query([playlist_id])?;
        let mut completed_count = 0_i64;
        while let Some(r) = rows.next()? {
            let id: String = r.get(0)?;
            let first_event = data.tracks.last().is_none_or(|t| t.id != id);
            if first_event {
                if let Some(last) = data.tracks.last_mut() {
                    last.score += REPEAT_BONUS * (completed_count as f64).ln_1p();
                }
                completed_count = 0;
                let genres_json: String = r.get(5)?;
                let genres: Vec<String> = serde_json::from_str(&genres_json)
                    .map_err(|e| Error::Internal(e.to_string()))?;
                data.tracks.push(ScoredTrack {
                    id,
                    title: r.get(1)?,
                    source_id: r.get(2)?,
                    artist_id: r.get(3)?,
                    artist_name: r.get(4)?,
                    genres,
                    score: 0.0,
                });
            }
            let started: i64 = r.get(8)?;
            let listened: i64 = r.get(9)?;
            let completed: bool = r.get(10)?;
            let skipped: bool = r.get(11)?;
            let duration: i64 = r.get(7)?;
            let fraction = if duration > 0 {
                (listened as f64 / duration as f64).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let age_days = (now.saturating_sub(started) as f64 / DAY_MS).max(0.0);
            let recency = 2.0_f64.powf(-age_days / RECENCY_HALF_LIFE_DAYS);
            let event = LISTEN_WEIGHT * fraction + if completed { COMPLETE_WEIGHT } else { 0.0 }
                - if skipped {
                    SKIP_BASE_PENALTY - SKIP_LATE_DISCOUNT * fraction
                } else {
                    0.0
                };
            let track = data.tracks.last_mut().expect("faixa recém-criada");
            track.score += event * recency;
            if first_event && r.get::<_, bool>(6)? {
                // O bônus de favorito é aplicado uma vez por faixa, no primeiro evento.
                track.score += FAVORITE_BONUS;
            }
            completed_count += i64::from(completed);
            data.total_listened_ms = data.total_listened_ms.saturating_add(listened);
            data.skipped += i64::from(skipped);
            data.ended += 1;
        }
        if let Some(last) = data.tracks.last_mut() {
            last.score += REPEAT_BONUS * (completed_count as f64).ln_1p();
        }
        for t in &data.tracks {
            if let (Some(id), Some(name)) = (&t.artist_id, &t.artist_name) {
                let entry = data
                    .artists
                    .entry(id.clone())
                    .or_insert((name.clone(), 0.0));
                entry.1 += t.score;
            }
            for genre in &t.genres {
                *data.genres.entry(genre.clone()).or_default() += t.score;
            }
        }
        Ok(data)
    }

    // @req RF-19
    /// Gosto global ou restrito às faixas atualmente numa playlist. O contexto do evento
    /// não limita o histórico: uma faixa da playlist leva todo o seu histórico consigo.
    pub fn taste_profile(&self, playlist_id: Option<&str>) -> Result<TasteProfile> {
        let data = self.taste_data(playlist_id)?;
        let mut top_artists: Vec<_> = data
            .artists
            .into_iter()
            .filter_map(|(artist_id, (name, score))| {
                (score > 0.0).then_some(TasteArtist {
                    artist_id,
                    name,
                    score,
                })
            })
            .collect();
        top_artists.sort_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then_with(|| a.name.cmp(&b.name))
        });
        let mut top_genres: Vec<_> = data
            .genres
            .into_iter()
            .filter_map(|(genre, score)| (score > 0.0).then_some(TasteGenre { genre, score }))
            .collect();
        top_genres.sort_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then_with(|| a.genre.cmp(&b.genre))
        });
        Ok(TasteProfile {
            top_artists,
            top_genres,
            total_listened_ms: data.total_listened_ms,
            skip_rate: if data.ended == 0 {
                0.0
            } else {
                data.skipped as f64 / data.ended as f64
            },
        })
    }

    // @req RF-19
    /// Sementes positivas para a casca buscar no YouTube; cada limite é independente.
    pub fn recommendation_seeds(
        &self,
        playlist_id: Option<&str>,
        track_limit: u32,
        artist_limit: u32,
        genre_limit: u32,
    ) -> Result<Vec<RecommendationSeed>> {
        let data = self.taste_data(playlist_id)?;
        let mut tracks: Vec<_> = data
            .tracks
            .iter()
            .filter(|t| t.score > 0.0 && t.source_id.is_some())
            .collect();
        tracks.sort_by(|a, b| b.score.total_cmp(&a.score));
        let mut out: Vec<_> = tracks
            .into_iter()
            .take(track_limit.min(100) as usize)
            .map(|t| RecommendationSeed {
                kind: RecommendationReasonKind::SimilarTrack,
                label: t.title.clone(),
                source_id: t.source_id.clone(),
                track_id: Some(t.id.clone()),
                score: t.score,
            })
            .collect();
        let mut artists: Vec<_> = data
            .artists
            .into_values()
            .filter(|(_, s)| *s > 0.0)
            .collect();
        artists.sort_by(|a, b| b.1.total_cmp(&a.1));
        out.extend(
            artists
                .into_iter()
                .take(artist_limit.min(100) as usize)
                .map(|(label, score)| RecommendationSeed {
                    kind: RecommendationReasonKind::Artist,
                    label,
                    source_id: None,
                    track_id: None,
                    score,
                }),
        );
        let mut genres: Vec<_> = data.genres.into_iter().filter(|(_, s)| *s > 0.0).collect();
        genres.sort_by(|a, b| b.1.total_cmp(&a.1));
        out.extend(
            genres
                .into_iter()
                .take(genre_limit.min(100) as usize)
                .map(|(label, score)| RecommendationSeed {
                    kind: RecommendationReasonKind::Genre,
                    label,
                    source_id: None,
                    track_id: None,
                    score,
                }),
        );
        Ok(out)
    }

    // @req RF-19
    /// Guarda/atualiza uma candidata. Devolve `false` se já está na discoteca ou dispensada.
    pub fn upsert_recommendation_candidate(&self, c: &RecommendationCandidate) -> Result<bool> {
        if c.source_id.trim().is_empty()
            || c.title.trim().is_empty()
            || c.artist.trim().is_empty()
            || c.seed_label.trim().is_empty()
            || c.duration_ms < 0
        {
            return Err(Error::invalid("candidata incompleta"));
        }
        if self
            .conn
            .query_row(
                "SELECT 1 FROM tracks WHERE source_id = ?1 LIMIT 1",
                [c.source_id.trim()],
                |_| Ok(()),
            )
            .optional()?
            .is_some()
        {
            return Ok(false);
        }
        let genres = serde_json::to_string(&normalized_genres(&c.genres))
            .map_err(|e| Error::Internal(e.to_string()))?;
        let n = self.conn.execute(
            "INSERT INTO recommendation_candidates (source_id, title, artist, duration_ms,
                thumbnail_url, genres, seed_kind, seed_label, seed_track_id, fetched_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(source_id) DO UPDATE SET title=excluded.title, artist=excluded.artist,
                duration_ms=excluded.duration_ms, thumbnail_url=excluded.thumbnail_url,
                genres=excluded.genres, seed_kind=excluded.seed_kind, seed_label=excluded.seed_label,
                seed_track_id=excluded.seed_track_id, fetched_at=excluded.fetched_at
             WHERE recommendation_candidates.dismissed = 0",
            params![c.source_id.trim(), c.title.trim(), c.artist.trim(), c.duration_ms,
                c.thumbnail_url, genres, c.seed_kind.as_str(), c.seed_label.trim(), c.seed_track_id,
                self.now_ms()],
        )?;
        Ok(n > 0)
    }

    // @req RF-19
    /// Grava uma lápide mesmo para ID desconhecido: a dispensa é permanente.
    pub fn dismiss_recommendation(&self, source_id: &str) -> Result<()> {
        if source_id.trim().is_empty() {
            return Err(Error::invalid("source_id vazio"));
        }
        self.conn.execute(
            "INSERT INTO recommendation_candidates (source_id, title, artist, duration_ms,
                seed_kind, seed_label, fetched_at, dismissed)
             VALUES (?1, '', '', 0, 'genre', '', ?2, 1)
             ON CONFLICT(source_id) DO UPDATE SET dismissed = 1",
            params![source_id.trim(), self.now_ms()],
        )?;
        Ok(())
    }

    // @req RF-19
    /// Ranking local; no máximo três candidatas por artista e score limitado a 0..1.
    pub fn list_recommendations(
        &self,
        playlist_id: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<Recommendation>> {
        let data = self.taste_data(playlist_id)?;
        let limit = limit.unwrap_or(20).min(100) as usize;
        if limit == 0 {
            return Ok(Vec::new());
        }
        if data.ended == 0 {
            return Ok(Vec::new());
        }
        let max_track = data
            .tracks
            .iter()
            .map(|t| t.score.max(0.0))
            .fold(0.0_f64, f64::max)
            .max(1.0);
        let max_artist = data
            .artists
            .values()
            .map(|(_, s)| s.max(0.0))
            .fold(0.0_f64, f64::max)
            .max(1.0);
        let max_genre = data
            .genres
            .values()
            .map(|s| s.max(0.0))
            .fold(0.0_f64, f64::max)
            .max(1.0);
        let mut stmt = self.conn.prepare(
            "SELECT c.source_id, c.title, c.artist, c.duration_ms, c.thumbnail_url,
                    c.genres, c.seed_kind, c.seed_label, c.seed_track_id
             FROM recommendation_candidates c
             WHERE c.dismissed = 0 AND NOT EXISTS (
                 SELECT 1 FROM tracks t WHERE t.source_id = c.source_id)",
        )?;
        let mut ranked = Vec::new();
        let mut rows = stmt.query([])?;
        while let Some(r) = rows.next()? {
            let artist: String = r.get(2)?;
            let genres_json: String = r.get(5)?;
            let genres: Vec<String> =
                serde_json::from_str(&genres_json).map_err(|e| Error::Internal(e.to_string()))?;
            let seed_kind: RecommendationReasonKind = r.get(6)?;
            let seed_label: String = r.get(7)?;
            let seed_track_id: Option<String> = r.get(8)?;
            let artist_score = data
                .artists
                .values()
                .filter(|(name, _)| name.eq_ignore_ascii_case(&artist))
                .map(|(_, score)| *score)
                .sum::<f64>()
                / max_artist;
            let genre_match = genres
                .iter()
                .filter_map(|g| data.genres.get(g).map(|s| (g, *s)))
                .max_by(|a, b| a.1.total_cmp(&b.1));
            let genre_score = genre_match.map(|(_, s)| s / max_genre).unwrap_or(0.0);
            let seed_score = match seed_kind {
                RecommendationReasonKind::SimilarTrack => data
                    .tracks
                    .iter()
                    .find(|t| Some(&t.id) == seed_track_id.as_ref())
                    .map(|t| t.score / max_track)
                    .unwrap_or(0.0),
                RecommendationReasonKind::Artist => data
                    .artists
                    .values()
                    .filter(|(name, _)| name.eq_ignore_ascii_case(&seed_label))
                    .map(|(_, s)| *s / max_artist)
                    .sum(),
                RecommendationReasonKind::Genre => data
                    .genres
                    .get(&seed_label.to_lowercase())
                    .map(|s| *s / max_genre)
                    .unwrap_or(0.0),
            };
            if artist_score <= 0.0 && genre_score <= 0.0 && seed_score <= 0.0 {
                continue;
            }
            let score = (0.15 + 0.45 * artist_score + 0.25 * genre_score + 0.35 * seed_score)
                .clamp(0.0, 1.0);
            let mut reasons = Vec::new();
            if seed_score > 0.0 {
                reasons.push((
                    seed_score * 0.35,
                    RecommendationReason {
                        kind: seed_kind,
                        label: seed_label,
                    },
                ));
            }
            if artist_score > 0.0 {
                reasons.push((
                    artist_score * 0.45,
                    RecommendationReason {
                        kind: RecommendationReasonKind::Artist,
                        label: artist.clone(),
                    },
                ));
            }
            if let Some((genre, value)) = genre_match.filter(|(_, s)| *s > 0.0) {
                reasons.push((
                    value / max_genre * 0.25,
                    RecommendationReason {
                        kind: RecommendationReasonKind::Genre,
                        label: genre.clone(),
                    },
                ));
            }
            reasons.sort_by(|a, b| b.0.total_cmp(&a.0));
            let mut seen = HashSet::new();
            reasons.retain(|(_, reason)| seen.insert((reason.kind.as_str(), reason.label.clone())));
            ranked.push(Recommendation {
                source_id: r.get(0)?,
                title: r.get(1)?,
                artist,
                duration_ms: r.get(3)?,
                thumbnail_url: r.get(4)?,
                score,
                reasons: reasons
                    .into_iter()
                    .take(3)
                    .map(|(_, reason)| reason)
                    .collect(),
            });
        }
        ranked.sort_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then_with(|| a.source_id.cmp(&b.source_id))
        });
        let mut by_artist: HashMap<String, usize> = HashMap::new();
        let mut out = Vec::new();
        for rec in ranked {
            let count = by_artist.entry(rec.artist.to_lowercase()).or_default();
            if *count >= MAX_PER_ARTIST {
                continue;
            }
            *count += 1;
            out.push(rec);
            if out.len() >= limit {
                break;
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{NewTrack, PlayEnd};
    use crate::testutil::{add_track, db};

    const NOW: i64 = 200 * 86_400_000;

    fn heard(db: &Database, id: &str, at: i64, end: PlayEnd) {
        let event = db.start_play_at(id, None, at).unwrap();
        db.add_listened(
            &event,
            if end == PlayEnd::Skipped {
                2_000
            } else {
                60_000
            },
        )
        .unwrap();
        db.finish_play(&event, end).unwrap();
    }

    fn candidate(source_id: &str, artist: &str, genre: &str) -> RecommendationCandidate {
        RecommendationCandidate {
            source_id: source_id.into(),
            title: source_id.into(),
            artist: artist.into(),
            duration_ms: 60_000,
            thumbnail_url: None,
            genres: vec![genre.into()],
            seed_kind: RecommendationReasonKind::Artist,
            seed_label: artist.into(),
            seed_track_id: None,
        }
    }

    // @test RF-19
    #[test]
    fn banco_vazio_e_dispensa_permanente() {
        let mut db = db();
        db.set_clock(|| NOW);
        assert!(db.taste_profile(None).unwrap().top_artists.is_empty());
        assert!(db.recommendation_seeds(None, 3, 3, 3).unwrap().is_empty());
        let c = candidate("x", "A", "rock");
        assert!(db.upsert_recommendation_candidate(&c).unwrap());
        assert!(db.list_recommendations(None, None).unwrap().is_empty());
        db.dismiss_recommendation("x").unwrap();
        assert!(!db.upsert_recommendation_candidate(&c).unwrap());
        db.dismiss_recommendation("future").unwrap();
        assert!(
            !db.upsert_recommendation_candidate(&candidate("future", "A", "rock"))
                .unwrap()
        );
    }

    // @test RF-19 RF-20
    #[test]
    fn ranking_reage_a_escuta_pulo_favorito_e_recencia() {
        let mut db = db();
        db.set_clock(|| NOW);
        let mut strong =
            NewTrack::youtube("strong-track", "Forte", Some("Forte".into()), 60_000, None);
        strong.genres = vec!["Rock".into(), " rock ".into()];
        let strong = db.upsert_track(&strong).unwrap().track;
        let weak = add_track(&db, "Fraca", "Fraca", 60_000);
        let old = add_track(&db, "Antiga", "Antiga", 60_000);
        for _ in 0..4 {
            heard(&db, &strong.id, NOW - 1_000, PlayEnd::Completed);
        }
        for _ in 0..4 {
            heard(&db, &weak.id, NOW - 1_000, PlayEnd::Skipped);
        }
        for _ in 0..4 {
            heard(&db, &old.id, NOW - 180 * 86_400_000, PlayEnd::Completed);
        }
        db.set_favorite(&strong.id, true).unwrap();
        assert_eq!(db.get_track(&strong.id).unwrap().genres, ["rock"]);
        for (sid, artist) in [("s", "Forte"), ("w", "Fraca"), ("o", "Antiga")] {
            db.upsert_recommendation_candidate(&candidate(sid, artist, "rock"))
                .unwrap();
        }
        let ranked = db.list_recommendations(None, None).unwrap();
        assert_eq!(ranked[0].source_id, "s");
        assert!(
            ranked.iter().position(|r| r.source_id == "o").unwrap()
                < ranked
                    .iter()
                    .position(|r| r.source_id == "w")
                    .unwrap_or(usize::MAX)
        );
        assert!(
            ranked
                .iter()
                .all(|r| (0.0..=1.0).contains(&r.score) && (1..=3).contains(&r.reasons.len()))
        );
        let profile = db.taste_profile(None).unwrap();
        assert!((profile.skip_rate - 1.0 / 3.0).abs() < 1e-8);
        assert_eq!(profile.top_artists[0].name, "Forte");
        assert_eq!(profile.top_genres[0].genre, "rock");
        let seeds = db.recommendation_seeds(None, 1, 1, 1).unwrap();
        assert_eq!(seeds.len(), 3);
        assert_eq!(seeds[0].source_id.as_deref(), Some("strong-track"));
    }

    // @test RF-19
    #[test]
    fn playlist_muda_ranking_e_discoteca_remove_candidata() {
        let mut db = db();
        db.set_clock(|| NOW);
        let a = add_track(&db, "Faixa A", "Artista A", 60_000);
        let b = add_track(&db, "Faixa B", "Artista B", 60_000);
        heard(&db, &a.id, NOW, PlayEnd::Completed);
        for _ in 0..3 {
            heard(&db, &b.id, NOW, PlayEnd::Completed);
        }
        let p = db.create_playlist("Só A", None).unwrap();
        db.add_tracks_to_playlist(&p.id, std::slice::from_ref(&a.id))
            .unwrap();
        db.upsert_recommendation_candidate(&candidate("ca", "Artista A", "x"))
            .unwrap();
        db.upsert_recommendation_candidate(&candidate("cb", "Artista B", "x"))
            .unwrap();
        assert_eq!(
            db.list_recommendations(None, None).unwrap()[0].source_id,
            "cb"
        );
        assert_eq!(
            db.list_recommendations(Some(&p.id), None).unwrap()[0].source_id,
            "ca"
        );
        assert_eq!(db.taste_profile(Some(&p.id)).unwrap().top_artists.len(), 1);
        assert_eq!(
            db.list_recommendations(Some(&p.id), Some(1)).unwrap().len(),
            1
        );
        db.upsert_track(&NewTrack::youtube("ca", "Na discoteca", None, 1, None))
            .unwrap();
        assert!(
            !db.upsert_recommendation_candidate(&candidate("ca", "Artista A", "x"))
                .unwrap()
        );
        assert!(
            db.list_recommendations(Some(&p.id), None)
                .unwrap()
                .is_empty()
        );
    }

    // @test RF-19
    #[test]
    fn favorito_e_recencia_mudam_a_ordem_e_diversidade_limita_artista() {
        let mut db = db();
        db.set_clock(|| NOW);
        let old = add_track(&db, "Velha", "Velha", 60_000);
        let fresh = add_track(&db, "Nova", "Nova", 60_000);
        heard(&db, &old.id, NOW - 90 * 86_400_000, PlayEnd::Completed);
        heard(&db, &fresh.id, NOW, PlayEnd::Completed);
        db.upsert_recommendation_candidate(&candidate("old", "Velha", "x"))
            .unwrap();
        db.upsert_recommendation_candidate(&candidate("fresh", "Nova", "x"))
            .unwrap();
        assert_eq!(
            db.list_recommendations(None, None).unwrap()[0].source_id,
            "fresh"
        );
        db.set_favorite(&old.id, true).unwrap();
        assert_eq!(
            db.list_recommendations(None, None).unwrap()[0].source_id,
            "old"
        );
        for n in 0..25 {
            db.upsert_recommendation_candidate(&candidate(&format!("new-{n}"), "Nova", "x"))
                .unwrap();
        }
        let all = db.list_recommendations(None, Some(100)).unwrap();
        assert_eq!(all.iter().filter(|r| r.artist == "Nova").count(), 3);
        assert!(db.list_recommendations(None, Some(0)).unwrap().is_empty());
    }
}
