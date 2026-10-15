//! Sorteio determinístico da fila. A casca fornece a semente e preserva a faixa atual.

use crate::model::{SurfleTrack, TasteGenre};

// @req RF-24
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }

    fn index(&mut self, count: usize) -> usize {
        let bound = count as u64;
        let threshold = bound.wrapping_neg() % bound;
        loop {
            let x = self.next();
            if x >= threshold {
                return (x % bound) as usize;
            }
        }
    }

    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / ((1_u64 << 53) as f64)
    }
}

// @req RF-24
/// Embaralhamento uniforme Fisher–Yates. Cada ocorrência da entrada aparece uma vez na saída.
pub fn random_order(track_ids: &[String], seed: u64) -> Vec<String> {
    let mut out = track_ids.to_vec();
    let mut rng = Rng(seed);
    for i in (1..out.len()).rev() {
        let j = rng.index(i + 1);
        out.swap(i, j);
    }
    out
}

// @req RF-24
fn same_artist(a: &SurfleTrack, b: &SurfleTrack) -> bool {
    a.artist_id.is_some() && a.artist_id == b.artist_id
}

// @req RF-24
/// Sorteio ponderado sem reposição. `current` é a faixa tocando; `remaining` só traz as demais.
/// Cada ocorrência mantém peso positivo; artistas recentes recebem penalidade decrescente.
pub fn surfle_order(
    current: &SurfleTrack,
    remaining: &[SurfleTrack],
    top_genres: &[TasteGenre],
    seed: u64,
) -> Vec<String> {
    let mut pool = remaining.to_vec();
    let max_score = pool
        .iter()
        .map(|t| t.score.max(0.0))
        .fold(0.0_f64, f64::max)
        .max(1.0);
    let mut rng = Rng(seed);
    let mut out = Vec::with_capacity(pool.len());
    let mut recent = vec![current.clone()];
    let max_genre = top_genres
        .iter()
        .map(|g| g.score.max(0.0))
        .fold(0.0_f64, f64::max)
        .max(1.0);
    while !pool.is_empty() {
        let previous = &recent[0];
        let has_alternative = pool.iter().any(|t| !same_artist(t, previous));
        let weights: Vec<f64> = pool
            .iter()
            .map(|t| {
                if has_alternative && same_artist(t, previous) {
                    return 0.0;
                }
                let genre_affinity = top_genres
                    .iter()
                    .filter(|g| t.genres.contains(&g.genre))
                    .map(|g| g.score.max(0.0) / max_genre)
                    .fold(0.0_f64, f64::max);
                let proximity = f64::from(t.genres.iter().any(|g| previous.genres.contains(g)));
                let base = 0.05
                    + 0.45 * (t.score.max(0.0) / max_score).min(1.0)
                    + 0.20 * proximity
                    + 0.30 * genre_affinity;
                let penalty = recent
                    .iter()
                    .enumerate()
                    .skip(1)
                    .take(2)
                    .filter(|(_, old)| same_artist(t, old))
                    .map(|(i, _)| if i == 1 { 0.25 } else { 0.5 })
                    .product::<f64>();
                base * penalty
            })
            .collect();
        let mut ticket = rng.unit() * weights.iter().sum::<f64>();
        let mut index = pool.len() - 1;
        for (i, weight) in weights.iter().enumerate() {
            if ticket < *weight {
                index = i;
                break;
            }
            ticket -= weight;
        }
        let chosen = pool.remove(index);
        out.push(chosen.track_id.clone());
        recent.insert(0, chosen);
        recent.truncate(3);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(id: &str, artist: &str, score: f64, genres: &[&str]) -> SurfleTrack {
        SurfleTrack {
            track_id: id.into(),
            artist_id: Some(artist.into()),
            score,
            genres: genres.iter().map(|s| (*s).into()).collect(),
        }
    }

    // @test RF-24
    #[test]
    fn embaralhamento_preserva_multiconjunto_e_varia_por_semente() {
        let ids: Vec<_> = (0..20).map(|n| n.to_string()).collect();
        let mut outputs = std::collections::HashSet::new();
        for seed in 0..100 {
            let mut out = random_order(&ids, seed);
            outputs.insert(out.clone());
            out.sort();
            let mut expected = ids.clone();
            expected.sort();
            assert_eq!(out, expected);
        }
        assert!(outputs.len() > 90);
    }

    // @test RF-24
    #[test]
    fn pontuacao_e_proximidade_afetam_posicao_media() {
        let current = track("current", "A", 0.0, &["rock"]);
        let remaining = [track("high", "X", 10.0, &[]), track("low", "Y", -10.0, &[])];
        let high_first = (0..1000)
            .filter(|seed| surfle_order(&current, &remaining, &[], *seed)[0] == "high")
            .count();
        assert!(high_first > 750, "{high_first}");
        let near = [
            track("near", "B", 0.0, &["rock"]),
            track("far", "C", 0.0, &[]),
        ];
        let near_first = (0..1000)
            .filter(|seed| surfle_order(&current, &near, &[], *seed)[0] == "near")
            .count();
        assert!(near_first > 700, "{near_first}");
    }

    // @test RF-24
    #[test]
    fn artista_dominante_alterna_enquanto_existe_alternativa() {
        let a = track("a", "A", 0.0, &[]);
        let choices = [
            track("a2", "A", 0.0, &[]),
            track("a3", "A", 0.0, &[]),
            track("b", "B", 0.0, &[]),
            track("c", "C", 0.0, &[]),
            track("d", "D", 0.0, &[]),
        ];
        for seed in 0..1000 {
            let out = surfle_order(&a, &choices, &[], seed);
            assert_eq!(out.len(), choices.len());
            for (i, id) in out.iter().enumerate() {
                let artist = choices.iter().find(|t| &t.track_id == id).unwrap();
                let previous = if i == 0 {
                    &a
                } else {
                    choices.iter().find(|t| t.track_id == out[i - 1]).unwrap()
                };
                if same_artist(artist, previous) {
                    assert!(
                        !out[i + 1..].iter().any(|later| choices
                            .iter()
                            .any(|t| &t.track_id == later && !same_artist(t, previous))),
                        "seed={seed}, {out:?}"
                    );
                }
            }
        }
    }

    // @test RF-24
    #[test]
    fn genero_favorito_aumenta_chance_sem_excluir_outras_faixas() {
        let current = track("current", "A", 0.0, &[]);
        let choices = [
            track("fav", "B", 0.0, &["rock"]),
            track("other", "C", 0.0, &["jazz"]),
        ];
        let taste = [
            TasteGenre {
                genre: "rock".into(),
                score: 8.0,
            },
            TasteGenre {
                genre: "jazz".into(),
                score: 1.0,
            },
        ];
        let first = (0..1000)
            .filter(|seed| surfle_order(&current, &choices, &taste, *seed)[0] == "fav")
            .count();
        assert!(first > 750 && first < 950, "{first}");
        assert!(surfle_order(&current, &choices, &taste, 0).len() == 2);
    }
}
