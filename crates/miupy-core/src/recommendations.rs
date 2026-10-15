//! Recomendação local. A rede e a busca de vídeos pertencem à casca; este módulo só lê e
//! grava SQLite e pontua candidatos com aritmética simples.

use std::collections::{HashMap, HashSet};

use rusqlite::{OptionalExtension, params};

use crate::db::Database;
use crate::error::{Error, Result};
use crate::model::{
    Dna, DnaTrend, DnaTrendKind, NewTrack, Recommendation, RecommendationCandidate,
    RecommendationReason, RecommendationReasonKind, RecommendationSeed, SurfleTrack, TasteArtist,
    TasteGenre, TasteProfile, TrackScore,
};
use crate::stats::PLAY_THRESHOLD_MS;
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
    plays: i64,
    skips: i64,
    last_played_at: Option<i64>,
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
        self.taste_data_window(playlist_id, None, None, self.now_ms(), true)
    }

    // @req RF-19 RF-25
    /// Mesmo cálculo para gosto, pontuação e DNA. Janelas usam início inclusivo e fim exclusivo.
    fn taste_data_window(
        &self,
        playlist_id: Option<&str>,
        start: Option<i64>,
        end: Option<i64>,
        now: i64,
        recency: bool,
    ) -> Result<TasteData> {
        if let Some(id) = playlist_id {
            self.get_playlist(id)?;
        }
        let mut data = TasteData::default();
        let mut stmt = self.conn.prepare(
            // @req RF-24: faixa sem gênero herda os do artista, como no Surfle.
            "SELECT t.id, t.title, t.source_id, t.artist_id, a.name,
                    CASE WHEN t.genres = '[]' THEN COALESCE(a.genres, '[]') ELSE t.genres END,
                    f.track_id IS NOT NULL, t.duration_ms,
                    e.started_at, e.listened_ms, e.completed, e.skipped
             FROM tracks t
             LEFT JOIN artists a ON a.id = t.artist_id
             LEFT JOIN favorites f ON f.track_id = t.id
             LEFT JOIN play_events e ON e.track_id = t.id AND e.ended_at IS NOT NULL
                 AND (?2 IS NULL OR e.started_at >= ?2)
                 AND (?3 IS NULL OR e.started_at < ?3)
             WHERE (?1 IS NULL OR EXISTS (
                 SELECT 1 FROM playlist_tracks pt WHERE pt.playlist_id = ?1 AND pt.track_id = t.id))
             ORDER BY t.id, e.started_at, e.id",
        )?;
        let mut rows = stmt.query(params![playlist_id, start, end])?;
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
                    score: if r.get::<_, bool>(6)? {
                        FAVORITE_BONUS
                    } else {
                        0.0
                    },
                    plays: 0,
                    skips: 0,
                    last_played_at: None,
                });
            }
            let Some(started) = r.get::<_, Option<i64>>(8)? else {
                continue;
            };
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
            let recency_weight = if recency {
                2.0_f64.powf(-age_days / RECENCY_HALF_LIFE_DAYS)
            } else {
                1.0
            };
            let event = LISTEN_WEIGHT * fraction + if completed { COMPLETE_WEIGHT } else { 0.0 }
                - if skipped {
                    SKIP_BASE_PENALTY - SKIP_LATE_DISCOUNT * fraction
                } else {
                    0.0
                };
            let track = data.tracks.last_mut().expect("faixa recém-criada");
            track.score += event * recency_weight;
            track.plays += i64::from(completed || listened >= PLAY_THRESHOLD_MS);
            track.skips += i64::from(skipped);
            track.last_played_at =
                Some(track.last_played_at.map_or(started, |old| old.max(started)));
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

    // @req RF-25
    /// Pontuação das faixas na ordem de ID; `None` inclui a biblioteca inteira. IDs ausentes
    /// são ignorados e IDs repetidos aparecem uma única vez.
    pub fn track_scores(&self, track_ids: Option<&[String]>) -> Result<Vec<TrackScore>> {
        let selected = track_ids.map(|ids| ids.iter().collect::<HashSet<_>>());
        Ok(self
            .taste_data(None)?
            .tracks
            .into_iter()
            .filter(|t| selected.as_ref().is_none_or(|ids| ids.contains(&t.id)))
            .map(|t| TrackScore {
                track_id: t.id,
                score: t.score,
                plays: t.plays,
                skips: t.skips,
                last_played_at: t.last_played_at,
            })
            .collect())
    }

    // @req RF-24
    /// Prepara as faixas para `surfle_order` na mesma ordem dos IDs pedidos. IDs ausentes
    /// são ignorados; a casca pode conferir a fila antes de sortear.
    pub fn surfle_tracks(&self, track_ids: &[String]) -> Result<Vec<SurfleTrack>> {
        let scores: HashMap<_, _> = self
            .track_scores(Some(track_ids))?
            .into_iter()
            .map(|s| (s.track_id, s.score))
            .collect();
        let mut artist_genres: HashMap<String, Vec<String>> = HashMap::new();
        let mut out = Vec::with_capacity(track_ids.len());
        for t in self.get_tracks_by_ids(track_ids)? {
            let genres = match (&t.artist_id, t.genres.is_empty()) {
                (Some(id), true) => match artist_genres.get(id) {
                    Some(g) => g.clone(),
                    None => {
                        let g = self.get_artist_info(id)?.genres;
                        artist_genres.insert(id.clone(), g.clone());
                        g
                    }
                },
                _ => t.genres,
            };
            out.push(SurfleTrack {
                score: scores.get(&t.id).copied().unwrap_or(0.0),
                track_id: t.id,
                artist_id: t.artist_id,
                genres,
            });
        }
        Ok(out)
    }

    // @req RF-25
    /// Compara duas janelas consecutivas. `None` usa 30 dias; aceita 7 a 365.
    pub fn dna(&self, window_days: Option<u32>, now: i64) -> Result<Dna> {
        let days = window_days.unwrap_or(30);
        if !(7..=365).contains(&days) {
            return Err(Error::invalid("janela do DNA deve ter 7 a 365 dias"));
        }
        let span = i64::from(days) * 86_400_000;
        let current =
            self.taste_data_window(None, Some(now.saturating_sub(span)), Some(now), now, false)?;
        let previous = self.taste_data_window(
            None,
            Some(now.saturating_sub(2 * span)),
            Some(now.saturating_sub(span)),
            now,
            false,
        )?;
        let mut top_artists: Vec<_> = current
            .artists
            .iter()
            .filter_map(|(id, (name, score))| {
                (*score > 0.0).then_some(TasteArtist {
                    artist_id: id.clone(),
                    name: name.clone(),
                    score: *score,
                })
            })
            .collect();
        top_artists.sort_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then_with(|| a.name.cmp(&b.name))
        });
        top_artists.truncate(10);
        let mut top_genres: Vec<_> = current
            .genres
            .iter()
            .filter_map(|(genre, score)| {
                (*score > 0.0).then_some(TasteGenre {
                    genre: genre.clone(),
                    score: *score,
                })
            })
            .collect();
        top_genres.sort_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then_with(|| a.genre.cmp(&b.genre))
        });
        top_genres.truncate(10);
        let mut trends = Vec::new();
        for (id, (name, current_score)) in &current.artists {
            let old = previous.artists.get(id).map_or(0.0, |(_, s)| *s);
            trends.push(DnaTrend {
                kind: DnaTrendKind::Artist,
                id: Some(id.clone()),
                label: name.clone(),
                current: *current_score,
                previous: old,
                delta: current_score - old,
            });
        }
        for (id, (name, old)) in &previous.artists {
            if !current.artists.contains_key(id) {
                trends.push(DnaTrend {
                    kind: DnaTrendKind::Artist,
                    id: Some(id.clone()),
                    label: name.clone(),
                    current: 0.0,
                    previous: *old,
                    delta: -old,
                });
            }
        }
        for (genre, current_score) in &current.genres {
            let old = previous.genres.get(genre).copied().unwrap_or(0.0);
            trends.push(DnaTrend {
                kind: DnaTrendKind::Genre,
                id: None,
                label: genre.clone(),
                current: *current_score,
                previous: old,
                delta: current_score - old,
            });
        }
        for (genre, old) in &previous.genres {
            if !current.genres.contains_key(genre) {
                trends.push(DnaTrend {
                    kind: DnaTrendKind::Genre,
                    id: None,
                    label: genre.clone(),
                    current: 0.0,
                    previous: *old,
                    delta: -old,
                });
            }
        }
        let mut rising: Vec<_> = trends.iter().filter(|t| t.delta >= 0.1).cloned().collect();
        rising.sort_by(|a, b| {
            b.delta
                .total_cmp(&a.delta)
                .then_with(|| a.label.cmp(&b.label))
        });
        rising.truncate(5);
        let mut falling: Vec<_> = trends.into_iter().filter(|t| t.delta <= -0.1).collect();
        falling.sort_by(|a, b| {
            a.delta
                .total_cmp(&b.delta)
                .then_with(|| a.label.cmp(&b.label))
        });
        falling.truncate(5);
        let rate = |d: &TasteData| {
            if d.ended == 0 {
                0.0
            } else {
                d.skipped as f64 / d.ended as f64
            }
        };
        Ok(Dna {
            window_days: days,
            top_artists,
            top_genres,
            skip_rate: rate(&current),
            previous_skip_rate: rate(&previous),
            listened_ms: current.total_listened_ms,
            previous_listened_ms: previous.total_listened_ms,
            rising,
            falling,
        })
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
    /// Faixa nova a partir de uma candidata não dispensada. A tela de recomendações sobrevive à
    /// reabertura do app, o cache de busca da casca não.
    pub fn recommendation_track(&self, source_id: &str) -> Result<Option<NewTrack>> {
        Ok(self
            .conn
            .query_row(
                "SELECT title, artist, duration_ms, thumbnail_url FROM recommendation_candidates
                 WHERE source_id = ?1 AND dismissed = 0",
                params![source_id],
                |r| {
                    let artist: String = r.get(1)?;
                    Ok(NewTrack::youtube(
                        source_id,
                        r.get::<_, String>(0)?,
                        (!artist.trim().is_empty()).then_some(artist),
                        r.get(2)?,
                        r.get(3)?,
                    ))
                },
            )
            .optional()?)
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
        if !data.tracks.iter().any(|t| t.score > 0.0) {
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
    fn candidata_vira_faixa_nova_e_dispensada_nao() {
        let db = db();
        db.upsert_recommendation_candidate(&candidate("y", "Banda", "rock"))
            .unwrap();
        let t = db.recommendation_track("y").unwrap().unwrap();
        assert_eq!(
            (
                t.source_id.as_deref(),
                t.title.as_str(),
                t.artist_name.as_deref()
            ),
            (Some("y"), "y", Some("Banda"))
        );
        assert_eq!(t.duration_ms, 60_000);
        db.dismiss_recommendation("y").unwrap();
        assert!(db.recommendation_track("y").unwrap().is_none());
        assert!(db.recommendation_track("nada").unwrap().is_none());
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

    // @test RF-19 RF-25
    #[test]
    fn favorita_sem_reproducao_alimenta_perfil_sementes_ranking_e_dna() {
        let mut db = db();
        db.set_clock(|| NOW);
        let mut n = NewTrack::youtube("fav-video", "Favorita", Some("Ana".into()), 60000, None);
        n.genres = vec!["rock".into()];
        let track = db.upsert_track(&n).unwrap().track;
        db.set_favorite(&track.id, true).unwrap();
        let profile = db.taste_profile(None).unwrap();
        assert_eq!(profile.top_artists[0].name, "Ana");
        assert_eq!(profile.top_genres[0].genre, "rock");
        assert_eq!(profile.total_listened_ms, 0);
        let seeds = db.recommendation_seeds(None, 1, 1, 1).unwrap();
        assert_eq!(seeds.len(), 3);
        assert_eq!(seeds[0].source_id.as_deref(), Some("fav-video"));
        db.upsert_recommendation_candidate(&candidate("other-video", "Ana", "rock"))
            .unwrap();
        assert_eq!(
            db.list_recommendations(None, None).unwrap()[0].source_id,
            "other-video"
        );
        let score = &db.track_scores(None).unwrap()[0];
        assert_eq!(
            (score.plays, score.skips, score.last_played_at),
            (0, 0, None)
        );
        assert_eq!(score.score, FAVORITE_BONUS);
        let dna = db.dna(None, NOW).unwrap();
        assert_eq!(dna.top_artists[0].score, FAVORITE_BONUS);
        assert!(dna.rising.is_empty(), "favorito entra em ambas as janelas");
    }

    // @test RF-25
    #[test]
    fn pontuacao_conta_plays_pulos_recencia_e_filtro() {
        let mut db = db();
        db.set_clock(|| NOW);
        let a = add_track(&db, "A", "Ana", 60000);
        let b = add_track(&db, "B", "Bia", 60000);
        heard(&db, &a.id, NOW - 1000, PlayEnd::Completed);
        heard(&db, &a.id, NOW - 500, PlayEnd::Skipped);
        let scores = db.track_scores(None).unwrap();
        assert_eq!(scores.len(), 2);
        let s = scores.iter().find(|s| s.track_id == a.id).unwrap();
        assert_eq!(
            (s.plays, s.skips, s.last_played_at),
            (1, 1, Some(NOW - 500))
        );
        assert!(s.score > 0.0 && s.score < 1.0);
        let empty = scores.iter().find(|s| s.track_id == b.id).unwrap();
        assert_eq!((empty.score, empty.plays), (0.0, 0));
        assert_eq!(
            db.track_scores(Some(&[b.id.clone(), b.id.clone(), "missing".into()]))
                .unwrap()
                .len(),
            1
        );
        let ready = db.surfle_tracks(&[a.id.clone(), b.id.clone()]).unwrap();
        assert_eq!(
            ready.iter().map(|t| &t.track_id).collect::<Vec<_>>(),
            [&a.id, &b.id]
        );
        assert_eq!(ready[0].score, s.score);
    }

    // @test RF-24
    #[test]
    fn surfle_usa_generos_do_artista_quando_faixa_nao_tem_genero() {
        let db = db();
        let track = add_track(&db, "Sem gênero", "Artista", 60000);
        assert!(track.genres.is_empty());
        db.conn
            .execute(
                "UPDATE artists SET genres='[\"rock\",\"samba\"]' WHERE id=?1",
                [track.artist_id.as_deref().unwrap()],
            )
            .unwrap();
        let prepared = db.surfle_tracks(std::slice::from_ref(&track.id)).unwrap();
        assert_eq!(prepared[0].genres, ["rock", "samba"]);

        // o gosto também herda: sem isso a afinidade de gênero do Surfle zera
        db.set_favorite(&track.id, true).unwrap();
        let genres: Vec<_> = db
            .taste_profile(None)
            .unwrap()
            .top_genres
            .into_iter()
            .map(|g| g.genre)
            .collect();
        assert!(genres.contains(&"rock".to_string()), "{genres:?}");
    }

    // @test RF-25
    #[test]
    fn dna_compara_janelas_limites_e_ausencia_de_eventos() {
        let mut db = db();
        db.set_clock(|| NOW);
        assert!(db.dna(None, NOW).unwrap().top_artists.is_empty());
        for days in [0, 6, 366] {
            assert_eq!(db.dna(Some(days), NOW).unwrap_err().code(), "invalid_input");
        }
        let old = add_track(&db, "Antiga", "Ana", 60000);
        let new = add_track(&db, "Nova", "Bia", 60000);
        heard(&db, &old.id, NOW - 45 * 86_400_000, PlayEnd::Completed);
        heard(&db, &new.id, NOW - 5 * 86_400_000, PlayEnd::Skipped);
        heard(&db, &new.id, NOW - 4 * 86_400_000, PlayEnd::Completed);
        let dna = db.dna(None, NOW).unwrap();
        assert_eq!(dna.window_days, 30);
        assert_eq!((dna.listened_ms, dna.previous_listened_ms), (62000, 60000));
        assert_eq!((dna.skip_rate, dna.previous_skip_rate), (0.5, 0.0));
        assert!(dna.rising.iter().any(|t| t.label == "Bia" && t.delta > 0.1));
        assert!(
            dna.falling
                .iter()
                .any(|t| t.label == "Ana" && t.delta < -0.1)
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
