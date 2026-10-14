//! Estatísticas da discoteca (RF-14) e retrospectiva anual (RF-15).
//!
//! # O que conta como "play"
//!
//! Um evento de reprodução é um *play* se foi **concluído** (`completed = 1`) **ou** teve
//! pelo menos [`PLAY_THRESHOLD_MS`] (30 s) de tempo realmente ouvido. Pular uma faixa aos
//! 10 s não é play; pular aos 40 s é. Faixas curtas (< 30 s) só contam se concluídas.
//!
//! - `totalPlays`, `plays` dos rankings, `distinctTracks` e `distinctArtists` usam só plays.
//! - `totalListenedMs`, `listenedMs` dos rankings, `listenedMsByMonth` e `busiestDay` somam
//!   **todo** o tempo ouvido, inclusive de eventos que não chegaram a ser play.
//! - Rankings ordenam por plays (desc), depois tempo ouvido (desc), depois título; faixas e
//!   artistas sem nenhum play ficam fora. Cada ranking tem no máximo [`TOP_LIMIT`] itens.
//!
//! # Fuso e datas
//!
//! O evento pertence ao dia/mês/ano do seu **início** (`started_at`), no fuso informado por
//! `tz_offset_minutes` (minutos a leste de UTC; Brasil/BRT = -180; no JS,
//! `-new Date().getTimezoneOffset()`). Um evento que começa às 23:59 do dia 31/12 e termina
//! depois da meia-noite fica inteiro no ano que acabou. O deslocamento é parâmetro para que
//! o resultado não dependa do relógio nem do fuso da máquina; horário de verão que mude no
//! meio do ano não é modelado (um único deslocamento vale para o ano todo).

use rusqlite::params;

use crate::db::Database;
use crate::error::{Error, Result};
use crate::model::{BusiestDay, LibraryStats, RankedArtist, RankedTrack, Wrapped};
use crate::time::year_bounds_ms;

/// Tempo mínimo ouvido para um evento não concluído contar como play.
pub const PLAY_THRESHOLD_MS: i64 = 30_000;

/// Tamanho máximo dos rankings (`topTracks`, `topArtists`).
pub const TOP_LIMIT: i64 = 10;

/// Condição SQL de "play" sobre a tabela `play_events` com alias `e`.
fn is_play() -> String {
    format!("(e.completed = 1 OR e.listened_ms >= {PLAY_THRESHOLD_MS})")
}

impl Database {
    // @req RF-14
    /// Totais da discoteca e ranking geral de faixas. `artistCount` conta artistas que têm
    /// ao menos uma faixa.
    pub fn library_stats(&self) -> Result<LibraryStats> {
        let count = |sql: &str| -> Result<i64> { Ok(self.conn.query_row(sql, [], |r| r.get(0))?) };
        let (total_listened_ms, total_plays) = self.listening_totals(i64::MIN, i64::MAX)?;
        Ok(LibraryStats {
            track_count: count("SELECT COUNT(*) FROM tracks")?,
            downloaded_count: count("SELECT COUNT(*) FROM tracks WHERE download_status = 'done'")?,
            playlist_count: count("SELECT COUNT(*) FROM playlists")?,
            artist_count: count("SELECT COUNT(DISTINCT artist_id) FROM tracks")?,
            favorite_count: count("SELECT COUNT(*) FROM favorites")?,
            total_duration_ms: count("SELECT COALESCE(SUM(duration_ms), 0) FROM tracks")?,
            total_listened_ms,
            total_plays,
            top_tracks: self.top_tracks(i64::MIN, i64::MAX)?,
        })
    }

    // @req RF-15
    /// Retrospectiva do ano civil `year` no fuso `tz_offset_minutes` (ver o módulo).
    pub fn wrapped(&self, year: i32, tz_offset_minutes: i32) -> Result<Wrapped> {
        if !(1970..=9999).contains(&year) {
            return Err(Error::invalid("ano fora do intervalo 1970..=9999"));
        }
        if tz_offset_minutes.abs() > 24 * 60 {
            return Err(Error::invalid("deslocamento de fuso fora de ±24 h"));
        }
        let (start, end) = year_bounds_ms(year, tz_offset_minutes);
        let off_ms = i64::from(tz_offset_minutes) * 60_000;
        let play = is_play();

        let (total_listened_ms, total_plays) = self.listening_totals(start, end)?;
        let (distinct_tracks, distinct_artists): (i64, i64) = self.conn.query_row(
            &format!(
                "SELECT COUNT(DISTINCT CASE WHEN {play} THEN e.track_id END),
                        COUNT(DISTINCT CASE WHEN {play} THEN t.artist_id END)
                 FROM play_events e JOIN tracks t ON t.id = e.track_id
                 WHERE e.started_at >= ?1 AND e.started_at < ?2"
            ),
            params![start, end],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        let tracks_added: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM tracks WHERE added_at >= ?1 AND added_at < ?2",
            params![start, end],
            |r| r.get(0),
        )?;

        let mut by_month = vec![0_i64; 12];
        {
            let mut stmt = self.conn.prepare(
                "SELECT CAST(strftime('%m', (e.started_at + ?3) / 1000, 'unixepoch') AS INTEGER),
                        SUM(e.listened_ms)
                 FROM play_events e
                 WHERE e.started_at >= ?1 AND e.started_at < ?2
                 GROUP BY 1",
            )?;
            let rows = stmt.query_map(params![start, end, off_ms], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?))
            })?;
            for row in rows {
                let (month, ms) = row?;
                if let Some(slot) = usize::try_from(month - 1)
                    .ok()
                    .and_then(|i| by_month.get_mut(i))
                {
                    *slot = ms;
                }
            }
        }

        let busiest_day = self
            .conn
            .query_row(
                "SELECT strftime('%Y-%m-%d', (e.started_at + ?3) / 1000, 'unixepoch') AS day,
                        SUM(e.listened_ms) AS ms
                 FROM play_events e
                 WHERE e.started_at >= ?1 AND e.started_at < ?2
                 GROUP BY day HAVING ms > 0
                 ORDER BY ms DESC, day ASC LIMIT 1",
                params![start, end, off_ms],
                |r| {
                    Ok(BusiestDay {
                        date: r.get(0)?,
                        listened_ms: r.get(1)?,
                    })
                },
            )
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })?;

        Ok(Wrapped {
            year,
            total_listened_ms,
            total_plays,
            distinct_tracks,
            distinct_artists,
            tracks_added,
            top_tracks: self.top_tracks(start, end)?,
            top_artists: self.top_artists(start, end)?,
            listened_ms_by_month: by_month,
            busiest_day,
        })
    }

    /// `(tempo ouvido, plays)` dos eventos que começaram em `[start, end)`.
    fn listening_totals(&self, start: i64, end: i64) -> Result<(i64, i64)> {
        Ok(self.conn.query_row(
            &format!(
                "SELECT COALESCE(SUM(e.listened_ms), 0),
                        COALESCE(SUM(CASE WHEN {} THEN 1 ELSE 0 END), 0)
                 FROM play_events e WHERE e.started_at >= ?1 AND e.started_at < ?2",
                is_play()
            ),
            params![start, end],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?)
    }

    fn top_tracks(&self, start: i64, end: i64) -> Result<Vec<RankedTrack>> {
        let sql = format!(
            "SELECT e.track_id,
                    SUM(CASE WHEN {} THEN 1 ELSE 0 END) AS plays,
                    SUM(e.listened_ms) AS listened
             FROM play_events e JOIN tracks t ON t.id = e.track_id
             WHERE e.started_at >= ?1 AND e.started_at < ?2
             GROUP BY e.track_id HAVING plays > 0
             ORDER BY plays DESC, listened DESC, t.title COLLATE miupy_nocase, e.track_id
             LIMIT ?3",
            is_play()
        );
        let rows: Vec<(String, i64, i64)> = {
            let mut stmt = self.conn.prepare(&sql)?;
            stmt.query_map(params![start, end, TOP_LIMIT], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            })?
            .collect::<rusqlite::Result<_>>()?
        };
        rows.into_iter()
            .map(|(id, plays, listened_ms)| {
                Ok(RankedTrack {
                    track: self.get_track(&id)?,
                    plays,
                    listened_ms,
                })
            })
            .collect()
    }

    fn top_artists(&self, start: i64, end: i64) -> Result<Vec<RankedArtist>> {
        let sql = format!(
            "SELECT a.id, a.name,
                    SUM(CASE WHEN {} THEN 1 ELSE 0 END) AS plays,
                    SUM(e.listened_ms) AS listened
             FROM play_events e
                  JOIN tracks t ON t.id = e.track_id
                  JOIN artists a ON a.id = t.artist_id
             WHERE e.started_at >= ?1 AND e.started_at < ?2
             GROUP BY a.id HAVING plays > 0
             ORDER BY plays DESC, listened DESC, a.name COLLATE miupy_nocase, a.id
             LIMIT ?3",
            is_play()
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![start, end, TOP_LIMIT], |r| {
            Ok(RankedArtist {
                artist_id: r.get(0)?,
                name: r.get(1)?,
                plays: r.get(2)?,
                listened_ms: r.get(3)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{NewTrack, PlayEnd};
    use crate::testutil::*;
    use crate::time::days_from_civil;

    const H: i64 = 3_600_000;
    const MIN: i64 = 60_000;

    /// Epoch ms de uma data/hora UTC.
    fn utc(y: i32, m: u32, d: u32, hh: i64, mm: i64) -> i64 {
        days_from_civil(y, m, d) * 86_400_000 + hh * H + mm * MIN
    }

    /// Registra um evento já fechado.
    fn play(db: &Database, track_id: &str, at: i64, listened: i64, completed: bool) {
        let ev = db.start_play_at(track_id, None, at).unwrap();
        db.add_listened(&ev, listened).unwrap();
        let end = if completed {
            PlayEnd::Completed
        } else {
            PlayEnd::Skipped
        };
        db.finish_play(&ev, end).unwrap();
    }

    // @test RF-14
    #[test]
    fn biblioteca_vazia_tem_tudo_zerado() {
        let db = db();
        let s = db.library_stats().unwrap();
        assert_eq!((s.track_count, s.playlist_count, s.artist_count), (0, 0, 0));
        assert_eq!((s.total_listened_ms, s.total_plays), (0, 0));
        assert!(s.top_tracks.is_empty());
        let w = db.wrapped(2026, 0).unwrap();
        assert_eq!(w.listened_ms_by_month, vec![0; 12]);
        assert_eq!(w.busiest_day, None);
        assert!(w.top_tracks.is_empty() && w.top_artists.is_empty());
    }

    // @test RF-14
    #[test]
    fn regra_de_play_30s_ou_concluida() {
        let db = db();
        let t = add_track(&db, "A", "X", 200_000);
        let t0 = utc(2026, 5, 1, 12, 0);
        play(&db, &t.id, t0, 29_999, false); // não é play
        play(&db, &t.id, t0 + 1, 30_000, false); // é play (limite)
        play(&db, &t.id, t0 + 2, 5_000, true); // concluída: é play
        let s = db.library_stats().unwrap();
        assert_eq!(s.total_plays, 2);
        assert_eq!(
            s.total_listened_ms,
            29_999 + 30_000 + 5_000,
            "tempo soma tudo"
        );
        assert_eq!(s.top_tracks.len(), 1);
        assert_eq!(
            (s.top_tracks[0].plays, s.top_tracks[0].listened_ms),
            (2, 64_999)
        );
    }

    // @test RF-14
    #[test]
    fn totais_da_discoteca() {
        let db = db();
        let a = add_track(&db, "A", "X", 100_000);
        let b = add_track(&db, "B", "X", 200_000);
        let c = add_track(&db, "C", "Y", 300_000);
        let _sem_artista = db.upsert_track(&NewTrack::local("/m/z.m4a", "Z")).unwrap();
        db.set_favorite(&a.id, true).unwrap();
        db.create_playlist("P", None).unwrap();
        db.mark_downloaded(
            &b.id,
            &crate::DownloadedFile {
                file_path: "/b".into(),
                codec: None,
                bitrate: None,
                cover_path: None,
                duration_ms: None,
            },
        )
        .unwrap();
        let s = db.library_stats().unwrap();
        assert_eq!(s.track_count, 4);
        assert_eq!(s.downloaded_count, 2, "b e a local");
        assert_eq!(s.playlist_count, 1);
        assert_eq!(s.artist_count, 2);
        assert_eq!(s.favorite_count, 1);
        assert_eq!(s.total_duration_ms, 600_000);

        // ranking: c com 2 plays, a com 1; b, sem eventos, fica de fora
        let t0 = utc(2026, 1, 1, 0, 0);
        play(&db, &c.id, t0, 60_000, true);
        play(&db, &c.id, t0 + 1, 60_000, true);
        play(&db, &a.id, t0 + 2, 90_000, false);
        let s = db.library_stats().unwrap();
        let ids: Vec<&str> = s.top_tracks.iter().map(|r| r.track.id.as_str()).collect();
        assert_eq!(ids, [c.id.as_str(), a.id.as_str()]);
        assert_eq!(s.top_tracks[0].track.title, "C");
        assert!(s.top_tracks[1].track.is_favorite);
    }

    // @test RF-15
    #[test]
    fn faixa_sem_nenhum_evento_nao_aparece_nem_quebra() {
        let mut db = db();
        let adicionada_em = utc(2026, 3, 1, 12, 0);
        db.set_clock(move || adicionada_em);
        let ouvida = add_track(&db, "Ouvida", "X", 100_000);
        let _muda = add_track(&db, "Muda", "Y", 100_000);
        play(&db, &ouvida.id, utc(2026, 3, 10, 12, 0), 60_000, true);
        let w = db.wrapped(2026, 0).unwrap();
        assert_eq!(w.top_tracks.len(), 1);
        assert_eq!(w.distinct_tracks, 1);
        assert_eq!(w.distinct_artists, 1);
        assert_eq!(w.tracks_added, 2);
    }

    // @test RF-15
    #[test]
    fn retrospectiva_completa_de_um_ano() {
        let db = db();
        let a = add_track(&db, "A", "Banda 1", 100_000);
        let b = add_track(&db, "B", "Banda 1", 100_000);
        let c = add_track(&db, "C", "Banda 2", 100_000);
        // jan: 2 plays de A no dia 5 (1h + 0,5h); fev: 1 play de B; dez: 1 play de C
        play(&db, &a.id, utc(2026, 1, 5, 10, 0), H, true);
        play(&db, &a.id, utc(2026, 1, 5, 15, 0), H / 2, true);
        play(&db, &b.id, utc(2026, 2, 20, 9, 0), 10 * MIN, true);
        play(&db, &c.id, utc(2026, 12, 31, 12, 0), 20 * MIN, true);
        // evento curto, sem play, em março: soma tempo, mas não conta play
        play(&db, &c.id, utc(2026, 3, 3, 9, 0), 10_000, false);
        // outro ano: não entra
        play(&db, &a.id, utc(2025, 6, 1, 9, 0), H, true);
        play(&db, &a.id, utc(2027, 1, 1, 9, 0), H, true);

        let w = db.wrapped(2026, 0).unwrap();
        assert_eq!(w.year, 2026);
        assert_eq!(
            w.total_listened_ms,
            H + H / 2 + 10 * MIN + 20 * MIN + 10_000
        );
        assert_eq!(w.total_plays, 4);
        assert_eq!((w.distinct_tracks, w.distinct_artists), (3, 2));
        assert_eq!(w.listened_ms_by_month.len(), 12);
        assert_eq!(w.listened_ms_by_month[0], H + H / 2);
        assert_eq!(w.listened_ms_by_month[1], 10 * MIN);
        assert_eq!(w.listened_ms_by_month[2], 10_000);
        assert_eq!(w.listened_ms_by_month[11], 20 * MIN);
        assert_eq!(w.listened_ms_by_month[3..11], [0; 8]);
        assert_eq!(
            w.busiest_day,
            Some(BusiestDay {
                date: "2026-01-05".into(),
                listened_ms: H + H / 2
            })
        );
        let tops: Vec<(&str, i64)> = w
            .top_tracks
            .iter()
            .map(|r| (r.track.title.as_str(), r.plays))
            .collect();
        assert_eq!(
            tops,
            [("A", 2), ("C", 1), ("B", 1)],
            "empate: mais tempo ouvido primeiro"
        );
        assert_eq!(w.top_tracks[0].listened_ms, H + H / 2);
        let artistas: Vec<(&str, i64)> = w
            .top_artists
            .iter()
            .map(|r| (r.name.as_str(), r.plays))
            .collect();
        assert_eq!(artistas, [("Banda 1", 3), ("Banda 2", 1)]);
        assert_eq!(w.top_artists[0].listened_ms, H + H / 2 + 10 * MIN);
    }

    // @test RF-15
    #[test]
    fn virada_de_ano_depende_do_fuso() {
        let db = db();
        let t = add_track(&db, "A", "X", 100_000);
        // 2027-01-01T02:30Z = 2026-12-31 23:30 em BRT (-180) = 2027-01-01 em UTC
        let noite = utc(2027, 1, 1, 2, 30);
        play(&db, &t.id, noite, 20 * MIN, true);
        // 2027-01-01T00:00Z exato: 2026-12-31 21:00 em BRT
        play(&db, &t.id, utc(2027, 1, 1, 0, 0), 5 * MIN, true);
        // 2026-01-01T01:00Z: ainda 2025-12-31 22:00 em BRT
        play(&db, &t.id, utc(2026, 1, 1, 1, 0), 7 * MIN, true);

        let brt_2026 = db.wrapped(2026, -180).unwrap();
        assert_eq!(
            brt_2026.total_listened_ms,
            25 * MIN,
            "os dois eventos da virada ainda são 2026"
        );
        assert_eq!(brt_2026.listened_ms_by_month[11], 25 * MIN);
        assert_eq!(
            brt_2026.busiest_day,
            Some(BusiestDay {
                date: "2026-12-31".into(),
                listened_ms: 25 * MIN
            })
        );
        let brt_2027 = db.wrapped(2027, -180).unwrap();
        assert_eq!(brt_2027.total_listened_ms, 0);

        let utc_2026 = db.wrapped(2026, 0).unwrap();
        assert_eq!(utc_2026.total_listened_ms, 7 * MIN);
        assert_eq!(utc_2026.listened_ms_by_month[0], 7 * MIN);
        let utc_2027 = db.wrapped(2027, 0).unwrap();
        assert_eq!(utc_2027.total_listened_ms, 25 * MIN);
        assert_eq!(utc_2027.busiest_day.unwrap().date, "2027-01-01");

        // fuso a leste: UTC+9 empurra 2026-12-31T20:00Z para 2027-01-01 05:00
        let t2 = add_track(&db, "B", "X", 1000);
        play(&db, &t2.id, utc(2026, 12, 31, 20, 0), MIN, true);
        assert_eq!(
            db.wrapped(2027, 9 * 60).unwrap().total_listened_ms,
            MIN + 25 * MIN
        );
        let w = db.wrapped(2026, 9 * 60).unwrap();
        assert_eq!(
            w.listened_ms_by_month[0],
            7 * MIN,
            "em UTC+9 o evento de 01:00Z vira 10:00 de 01/01"
        );
        assert_eq!(w.listened_ms_by_month[11], 0);
    }

    // @test RF-15
    #[test]
    fn dia_mais_movimentado_desempata_pelo_dia_mais_cedo_e_tracks_added_usa_fuso() {
        let mut db = db();
        let t = add_track(&db, "A", "X", 1000);
        play(&db, &t.id, utc(2026, 4, 2, 10, 0), MIN, true);
        play(&db, &t.id, utc(2026, 4, 1, 10, 0), MIN, true);
        assert_eq!(
            db.wrapped(2026, 0).unwrap().busiest_day.unwrap().date,
            "2026-04-01"
        );

        // faixa adicionada 2027-01-01T01:00Z: 2026 em BRT, 2027 em UTC
        let adicionada_em = utc(2027, 1, 1, 1, 0);
        db.set_clock(move || adicionada_em);
        add_track(&db, "Nova", "X", 1000);
        assert_eq!(db.wrapped(2026, -180).unwrap().tracks_added, 1);
        assert_eq!(db.wrapped(2027, -180).unwrap().tracks_added, 0);
        assert_eq!(db.wrapped(2027, 0).unwrap().tracks_added, 1);
    }

    // @test RF-15
    #[test]
    fn valida_ano_e_fuso() {
        let db = db();
        assert_eq!(db.wrapped(1800, 0).unwrap_err().code(), "invalid_input");
        assert_eq!(db.wrapped(2026, 5000).unwrap_err().code(), "invalid_input");
    }

    // @test RF-14
    #[test]
    fn ranking_limita_a_dez() {
        let db = db();
        for i in 0..12 {
            let t = add_track(&db, &format!("T{i:02}"), "X", 1000);
            play(&db, &t.id, utc(2026, 6, 1, 12, 0) + i, 60_000, true);
        }
        let s = db.library_stats().unwrap();
        assert_eq!(s.top_tracks.len(), 10);
        assert_eq!(db.wrapped(2026, 0).unwrap().top_tracks.len(), 10);
    }
}
