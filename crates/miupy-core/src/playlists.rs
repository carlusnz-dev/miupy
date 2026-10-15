//! Playlists: CRUD, itens com posição contígua, reordenação.

use std::collections::HashSet;

use rusqlite::{OptionalExtension, Row, params};
use serde_json::json;

use crate::db::{Database, new_id};
use crate::error::{Error, Result};
use crate::model::{Playlist, PlaylistPatch, Track, TrackQuery};
use crate::text::clean_opt;

const MAX_NAME_CHARS: usize = 200;

const PLAYLIST_SQL: &str = "SELECT p.id, p.name, p.description, p.cover_path,
        (SELECT COUNT(*) FROM playlist_tracks pt WHERE pt.playlist_id = p.id),
        (SELECT COALESCE(SUM(t.duration_ms), 0) FROM playlist_tracks pt
            JOIN tracks t ON t.id = pt.track_id WHERE pt.playlist_id = p.id),
        p.created_at, p.updated_at,
        COALESCE(p.id = (SELECT favorites_playlist_id FROM settings WHERE id = 1), 0),
        p.auto_download
     FROM playlists p";

fn playlist_from_row(r: &Row<'_>) -> rusqlite::Result<Playlist> {
    Ok(Playlist {
        id: r.get(0)?,
        name: r.get(1)?,
        description: r.get(2)?,
        cover_path: r.get(3)?,
        mosaic_paths: Vec::new(),
        track_count: r.get(4)?,
        total_duration_ms: r.get(5)?,
        created_at: r.get(6)?,
        updated_at: r.get(7)?,
        is_favorites: r.get(8)?,
        auto_download: r.get(9)?,
    })
}

fn valid_name(name: &str) -> Result<String> {
    let n = name.trim();
    if n.is_empty() {
        return Err(Error::invalid("o nome da playlist não pode ser vazio"));
    }
    if n.chars().count() > MAX_NAME_CHARS {
        return Err(Error::invalid(format!(
            "o nome passa de {MAX_NAME_CHARS} caracteres"
        )));
    }
    Ok(n.to_owned())
}

impl Database {
    // @req RF-06
    pub fn create_playlist(&self, name: &str, description: Option<&str>) -> Result<Playlist> {
        let name = valid_name(name)?;
        let id = new_id();
        let now = self.now_ms();
        self.in_tx(|| {
            self.conn.execute(
                "INSERT INTO playlists (id, name, description, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?4)",
                params![id, name, clean_opt(description), now],
            )?;
            self.record_action("playlist_created", Some("playlist"), Some(&id), None)?;
            self.get_playlist(&id)
        })
    }

    // @req RF-06
    pub fn get_playlist(&self, playlist_id: &str) -> Result<Playlist> {
        let mut playlist = self
            .conn
            .query_row(
                &format!("{PLAYLIST_SQL} WHERE p.id = ?1"),
                [playlist_id],
                playlist_from_row,
            )
            .optional()?
            .ok_or_else(|| Error::not_found("playlist", playlist_id))?;
        playlist.mosaic_paths = self.playlist_mosaic_paths(playlist_id)?;
        Ok(playlist)
    }

    // @req RF-06
    /// Todas as playlists, na ordem de criação, com `trackCount` e `totalDurationMs`.
    pub fn list_playlists(&self) -> Result<Vec<Playlist>> {
        let mut stmt = self
            .conn
            .prepare(&format!("{PLAYLIST_SQL} ORDER BY p.created_at, p.id"))?;
        let rows = stmt.query_map([], playlist_from_row)?;
        let mut playlists: Vec<Playlist> = rows.collect::<rusqlite::Result<_>>()?;
        for playlist in &mut playlists {
            playlist.mosaic_paths = self.playlist_mosaic_paths(&playlist.id)?;
        }
        Ok(playlists)
    }

    // @req RF-30
    /// Marca outra playlist; preserva a ordem da antiga ao copiar faixas ainda ausentes.
    pub fn favorites_set_playlist(&self, playlist_id: &str) -> Result<Playlist> {
        self.in_tx(|| {
            self.get_playlist(playlist_id)?;
            let old = self.get_settings()?.favorites_playlist_id;
            if old.as_deref() == Some(playlist_id) {
                return self.get_playlist(playlist_id);
            }
            if let Some(old_id) = old {
                let ids: Vec<String> = self.conn.prepare(
                    "SELECT track_id FROM playlist_tracks WHERE playlist_id = ?1 ORDER BY position, track_id"
                )?.query_map([&old_id], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?;
                self.add_tracks_to_playlist(playlist_id, &ids)?;
            }
            self.conn.execute(
                "UPDATE settings SET favorites_playlist_id = ?1, updated_at = ?2 WHERE id = 1",
                params![playlist_id, self.now_ms()],
            )?;
            self.get_playlist(playlist_id)
        })
    }

    // @req RF-22
    /// Até quatro imagens, uma por artista: mais faixas primeiro, empate por nome.
    /// Foto do artista tem prioridade; sem foto, usa uma capa da faixa na playlist.
    pub fn playlist_mosaic_paths(&self, playlist_id: &str) -> Result<Vec<String>> {
        self.conn
            .query_row(
                "SELECT 1 FROM playlists WHERE id = ?1",
                [playlist_id],
                |r| r.get::<_, i64>(0),
            )
            .optional()?
            .ok_or_else(|| Error::not_found("playlist", playlist_id))?;
        let mut stmt = self.conn.prepare(
            "WITH ranked AS (
                 SELECT a.id, a.name, a.image_path, COUNT(*) AS track_count
                 FROM playlist_tracks pt
                 JOIN tracks t ON t.id = pt.track_id
                 JOIN artists a ON a.id = t.artist_id
                 WHERE pt.playlist_id = ?1
                 GROUP BY a.id
                 ORDER BY track_count DESC, a.name COLLATE miupy_nocase, a.id
                 LIMIT 4
             )
             SELECT COALESCE(r.image_path, (
                 SELECT t2.cover_path FROM playlist_tracks pt2
                 JOIN tracks t2 ON t2.id = pt2.track_id
                 WHERE pt2.playlist_id = ?1 AND t2.artist_id = r.id AND t2.cover_path IS NOT NULL
                 ORDER BY pt2.position, t2.id LIMIT 1)) AS image
             FROM ranked r
             WHERE COALESCE(r.image_path, (
                 SELECT t2.cover_path FROM playlist_tracks pt2
                 JOIN tracks t2 ON t2.id = pt2.track_id
                 WHERE pt2.playlist_id = ?1 AND t2.artist_id = r.id AND t2.cover_path IS NOT NULL
                 ORDER BY pt2.position, t2.id LIMIT 1)) IS NOT NULL
             ORDER BY r.track_count DESC, r.name COLLATE miupy_nocase, r.id",
        )?;
        Ok(stmt
            .query_map([playlist_id], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?)
    }

    // @req RF-22
    /// Salva ou remove a capa escolhida e devolve o caminho anterior.
    pub fn set_playlist_cover(
        &self,
        playlist_id: &str,
        cover_path: Option<&str>,
    ) -> Result<Option<String>> {
        let old = self.get_playlist(playlist_id)?.cover_path;
        self.conn.execute(
            "UPDATE playlists SET cover_path = ?2, updated_at = ?3 WHERE id = ?1",
            params![playlist_id, clean_opt(cover_path), self.now_ms()],
        )?;
        Ok(old)
    }

    // @req RF-06
    pub fn update_playlist(&self, playlist_id: &str, patch: &PlaylistPatch) -> Result<Playlist> {
        self.in_tx(|| {
            let cur = self.get_playlist(playlist_id)?;
            let name = match &patch.name {
                Some(n) => valid_name(n)?,
                None => cur.name,
            };
            let description = match &patch.description {
                Some(d) => clean_opt(d.as_deref()),
                None => cur.description,
            };
            let cover = match &patch.cover_path {
                Some(c) => clean_opt(c.as_deref()),
                None => cur.cover_path,
            };
            self.conn.execute(
                "UPDATE playlists SET name = ?2, description = ?3, cover_path = ?4,
                        updated_at = ?5, auto_download = ?6 WHERE id = ?1",
                params![
                    playlist_id,
                    name,
                    description,
                    cover,
                    self.now_ms(),
                    patch.auto_download.unwrap_or(cur.auto_download)
                ],
            )?;
            self.get_playlist(playlist_id)
        })
    }

    // @req RF-31
    /// IDs sem arquivo que podem entrar na fila automática, em ordem da playlist.
    pub fn playlist_auto_pending_tracks(&self, playlist_id: &str) -> Result<Vec<String>> {
        self.get_playlist(playlist_id)?;
        let mut stmt = self.conn.prepare(
            "SELECT t.id FROM playlist_tracks pt JOIN tracks t ON t.id = pt.track_id
             WHERE pt.playlist_id = ?1 AND t.download_status IN ('none', 'error')
             ORDER BY pt.position, t.id",
        )?;
        Ok(stmt
            .query_map([playlist_id], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?)
    }

    // @req RF-31
    /// Playlists automáticas que contêm a faixa, inclusive a de favoritas.
    pub fn auto_playlists_for_track(&self, track_id: &str) -> Result<Vec<String>> {
        self.require_track(track_id)?;
        let mut stmt = self.conn.prepare(
            "SELECT p.id FROM playlists p JOIN playlist_tracks pt ON pt.playlist_id = p.id
             WHERE pt.track_id = ?1 AND p.auto_download = 1 ORDER BY p.created_at, p.id",
        )?;
        Ok(stmt
            .query_map([track_id], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?)
    }

    // @req RF-31
    pub fn auto_playlist_ids(&self) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM playlists WHERE auto_download = 1 ORDER BY created_at, id")?;
        Ok(stmt
            .query_map([], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?)
    }

    // @req RF-06
    /// Apaga a playlist e seus itens (as faixas continuam na biblioteca).
    pub fn delete_playlist(&self, playlist_id: &str) -> Result<()> {
        self.in_tx(|| {
            let n = self
                .conn
                .execute("DELETE FROM playlists WHERE id = ?1", [playlist_id])?;
            if n == 0 {
                return Err(Error::not_found("playlist", playlist_id));
            }
            self.record_action(
                "playlist_deleted",
                Some("playlist"),
                Some(playlist_id),
                None,
            )?;
            Ok(())
        })
    }

    // @req RF-07 RF-08
    /// Faixas da playlist, filtradas e ordenadas. Sem `sortBy`: por posição.
    pub fn playlist_tracks(&self, playlist_id: &str, query: &TrackQuery) -> Result<Vec<Track>> {
        self.get_playlist(playlist_id)?;
        self.select_tracks(Some(playlist_id), None, query)
    }

    // @req RF-06
    /// Acrescenta ao fim, na ordem dada. Faixas já presentes (ou repetidas na lista) são
    /// ignoradas. Faixa inexistente aborta tudo com `not_found`.
    pub fn add_tracks_to_playlist(
        &self,
        playlist_id: &str,
        track_ids: &[String],
    ) -> Result<Playlist> {
        self.in_tx(|| {
            self.get_playlist(playlist_id)?;
            for tid in track_ids {
                self.require_track(tid)?;
            }
            let mut next: i64 = self.conn.query_row(
                "SELECT COALESCE(MAX(position) + 1, 0) FROM playlist_tracks WHERE playlist_id = ?1",
                [playlist_id],
                |r| r.get(0),
            )?;
            let now = self.now_ms();
            let mut seen = HashSet::new();
            let mut added = 0;
            for tid in track_ids {
                if !seen.insert(tid) {
                    continue;
                }
                let n = self.conn.execute(
                    "INSERT OR IGNORE INTO playlist_tracks (playlist_id, track_id, position, added_at)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![playlist_id, tid, next, now],
                )?;
                if n > 0 {
                    next += 1;
                    added += 1;
                }
            }
            if added > 0 {
                self.touch_playlist(playlist_id)?;
                self.record_action(
                    "playlist_tracks_added",
                    Some("playlist"),
                    Some(playlist_id),
                    Some(&json!({ "count": added })),
                )?;
            }
            self.get_playlist(playlist_id)
        })
    }

    // @req RF-06
    /// Remove as faixas da playlist (não da biblioteca) e recompacta as posições. Faixas que
    /// não estavam na playlist são ignoradas.
    pub fn remove_tracks_from_playlist(
        &self,
        playlist_id: &str,
        track_ids: &[String],
    ) -> Result<Playlist> {
        self.in_tx(|| {
            self.get_playlist(playlist_id)?;
            let mut removed = 0;
            for tid in track_ids {
                removed += self.conn.execute(
                    "DELETE FROM playlist_tracks WHERE playlist_id = ?1 AND track_id = ?2",
                    params![playlist_id, tid],
                )?;
            }
            if removed > 0 {
                self.renumber_playlist(playlist_id)?;
                self.touch_playlist(playlist_id)?;
                self.record_action(
                    "playlist_tracks_removed",
                    Some("playlist"),
                    Some(playlist_id),
                    Some(&json!({ "count": removed })),
                )?;
            }
            self.get_playlist(playlist_id)
        })
    }

    // @req RF-07
    /// Move a faixa para `new_position` (0-based), empurrando as demais e mantendo as posições
    /// em `0..n-1`. Posição acima do fim é limitada ao último lugar.
    pub fn move_playlist_track(
        &self,
        playlist_id: &str,
        track_id: &str,
        new_position: i64,
    ) -> Result<()> {
        if new_position < 0 {
            return Err(Error::invalid("a posição não pode ser negativa"));
        }
        self.in_tx(|| {
            self.get_playlist(playlist_id)?;
            let mut order: Vec<String> = {
                let mut stmt = self.conn.prepare(
                    "SELECT track_id FROM playlist_tracks WHERE playlist_id = ?1
                     ORDER BY position, added_at, track_id",
                )?;
                stmt.query_map([playlist_id], |r| r.get(0))?
                    .collect::<rusqlite::Result<_>>()?
            };
            let from = order
                .iter()
                .position(|t| t == track_id)
                .ok_or_else(|| Error::not_found("faixa na playlist", track_id))?;
            let item = order.remove(from);
            let to = usize::try_from(new_position)
                .unwrap_or(usize::MAX)
                .min(order.len());
            order.insert(to, item);
            let mut upd = self.conn.prepare(
                "UPDATE playlist_tracks SET position = ?3 WHERE playlist_id = ?1 AND track_id = ?2",
            )?;
            for (pos, tid) in order.iter().enumerate() {
                upd.execute(params![playlist_id, tid, pos as i64])?;
            }
            self.touch_playlist(playlist_id)
        })
    }

    /// Reescreve as posições como 0..n-1 preservando a ordem atual.
    pub(crate) fn renumber_playlist(&self, playlist_id: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE playlist_tracks AS pt SET position = r.rn
             FROM (SELECT track_id,
                          ROW_NUMBER() OVER (ORDER BY position, added_at, track_id) - 1 AS rn
                   FROM playlist_tracks WHERE playlist_id = ?1) AS r
             WHERE pt.playlist_id = ?1 AND pt.track_id = r.track_id",
            [playlist_id],
        )?;
        Ok(())
    }

    pub(crate) fn touch_playlist(&self, playlist_id: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE playlists SET updated_at = ?2 WHERE id = ?1",
            params![playlist_id, self.now_ms()],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{SortDir, TrackSort};
    use crate::testutil::*;

    // @test RF-31
    #[test]
    fn auto_download_persiste_e_consulta_apenas_faixas_elegiveis() {
        let db = db();
        let p = db.create_playlist("Auto", None).unwrap();
        assert!(!p.auto_download);
        let a = db
            .upsert_track(&crate::NewTrack::youtube("auto-a", "A", None, 1000, None))
            .unwrap()
            .track;
        let b = db
            .upsert_track(&crate::NewTrack::youtube("auto-b", "B", None, 1000, None))
            .unwrap()
            .track;
        let local = db
            .upsert_track(&crate::NewTrack::local("/tmp/auto-local.m4a", "Local"))
            .unwrap()
            .track;
        db.add_tracks_to_playlist(&p.id, &[a.id.clone(), local.id.clone(), b.id.clone()])
            .unwrap();
        let other = db.create_playlist("Outra", None).unwrap();
        db.add_tracks_to_playlist(&other.id, std::slice::from_ref(&a.id))
            .unwrap();
        assert!(db.auto_playlists_for_track(&a.id).unwrap().is_empty());
        let p = db
            .update_playlist(
                &p.id,
                &PlaylistPatch {
                    auto_download: Some(true),
                    ..Default::default()
                },
            )
            .unwrap();
        assert!(p.auto_download);
        assert_eq!(
            db.playlist_auto_pending_tracks(&p.id).unwrap(),
            vec![a.id.clone(), b.id.clone()]
        );
        assert_eq!(
            db.auto_playlists_for_track(&a.id).unwrap(),
            vec![p.id.clone()]
        );
        let jobs = db.enqueue_downloads(&[a.id.clone(), b.id.clone()]).unwrap();
        db.update_download(
            &jobs[1].id,
            &crate::DownloadUpdate {
                status: crate::DownloadStatus::Error,
                progress: None,
                error: Some("falha".into()),
            },
        )
        .unwrap();
        assert_eq!(
            db.playlist_auto_pending_tracks(&p.id).unwrap(),
            vec![b.id.clone()]
        );
        db.update_playlist(
            &p.id,
            &PlaylistPatch {
                auto_download: Some(false),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(db.auto_playlists_for_track(&a.id).unwrap().is_empty());
        assert!(
            db.list_downloads()
                .unwrap()
                .iter()
                .any(|j| j.track_id == a.id)
        );
    }

    fn positions(db: &Database, pid: &str) -> Vec<(String, i64)> {
        let mut stmt = db
            .conn
            .prepare(
                "SELECT t.title, pt.position FROM playlist_tracks pt
                 JOIN tracks t ON t.id = pt.track_id WHERE pt.playlist_id = ?1 ORDER BY pt.position",
            )
            .unwrap();
        stmt.query_map([pid], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap()
    }

    fn playlist_abcd(db: &Database) -> (Playlist, Vec<Track>) {
        let ts: Vec<Track> = ["A", "B", "C", "D"]
            .iter()
            .map(|t| add_track(db, t, "Art", 60_000))
            .collect();
        let ids: Vec<String> = ts.iter().map(|t| t.id.clone()).collect();
        let pl = db.create_playlist("Mix", None).unwrap();
        let pl = db.add_tracks_to_playlist(&pl.id, &ids).unwrap();
        (pl, ts)
    }

    // @test RF-06
    #[test]
    fn crud_de_playlist_com_validacao() {
        let mut db = db();
        let pl = db.create_playlist("  Rock  ", Some(" clássicos ")).unwrap();
        assert_eq!(pl.name, "Rock");
        assert_eq!(pl.description.as_deref(), Some("clássicos"));
        assert_eq!((pl.track_count, pl.total_duration_ms), (0, 0));
        assert_eq!(
            db.create_playlist("  ", None).unwrap_err().code(),
            "invalid_input"
        );

        db.set_clock(|| 5_000);
        let up = db
            .update_playlist(
                &pl.id,
                &PlaylistPatch {
                    name: Some("Rock BR".into()),
                    description: Some(None),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(up.name, "Rock BR");
        assert_eq!(up.description, None);
        assert_eq!(up.updated_at, 5_000);
        let mantem = db
            .update_playlist(&pl.id, &PlaylistPatch::default())
            .unwrap();
        assert_eq!(mantem.name, "Rock BR");
        let bad = PlaylistPatch {
            name: Some("".into()),
            ..Default::default()
        };
        assert_eq!(
            db.update_playlist(&pl.id, &bad).unwrap_err().code(),
            "invalid_input"
        );

        assert_eq!(db.list_playlists().unwrap().len(), 2);
        db.delete_playlist(&pl.id).unwrap();
        assert_eq!(db.delete_playlist(&pl.id).unwrap_err().code(), "not_found");
        assert_eq!(db.list_playlists().unwrap().len(), 1);
    }

    // @test RF-04 RF-30
    #[test]
    fn trocar_favoritas_copia_sem_repetir_e_apagar_limpa_marcacao() {
        let db = db();
        let a = db
            .upsert_track(&crate::NewTrack::youtube("a", "A", None, 1, None))
            .unwrap()
            .track;
        let b = db
            .upsert_track(&crate::NewTrack::youtube("b", "B", None, 1, None))
            .unwrap()
            .track;
        let c = db
            .upsert_track(&crate::NewTrack::youtube("c", "C", None, 1, None))
            .unwrap()
            .track;
        let old = db.get_settings().unwrap().favorites_playlist_id.unwrap();
        db.set_favorite(&a.id, true).unwrap();
        db.set_favorite(&b.id, true).unwrap();
        let next = db.create_playlist("Nova", None).unwrap();
        db.add_tracks_to_playlist(&next.id, &[b.id.clone(), c.id.clone()])
            .unwrap();
        let marked = db.favorites_set_playlist(&next.id).unwrap();
        assert!(marked.is_favorites);
        assert!(!db.get_playlist(&old).unwrap().is_favorites);
        let ids: Vec<String> = db
            .playlist_tracks(&next.id, &TrackQuery::default())
            .unwrap()
            .into_iter()
            .map(|t| t.id)
            .collect();
        assert_eq!(ids, vec![b.id.clone(), c.id, a.id.clone()]);
        assert_eq!(db.favorites_set_playlist(&next.id).unwrap().track_count, 3);
        assert_eq!(
            db.favorites_set_playlist("missing").unwrap_err().code(),
            "not_found"
        );
        db.set_favorite(&a.id, false).unwrap();
        assert!(!db.get_track(&a.id).unwrap().is_favorite);
        assert_eq!(db.get_playlist(&old).unwrap().track_count, 2);
        db.delete_playlist(&next.id).unwrap();
        assert_eq!(db.get_settings().unwrap().favorites_playlist_id, None);
        assert!(!db.get_track(&b.id).unwrap().is_favorite);
        db.set_favorite(&b.id, false).unwrap();
        assert_eq!(db.get_settings().unwrap().favorites_playlist_id, None);
        db.set_favorite(&b.id, true).unwrap();
        assert!(db.get_settings().unwrap().favorites_playlist_id.is_some());
        assert!(db.get_track(&b.id).unwrap().is_favorite);
    }

    // @test RF-06
    #[test]
    fn adiciona_sem_duplicar_e_calcula_totais() {
        let db = db();
        let (pl, ts) = playlist_abcd(&db);
        assert_eq!(pl.track_count, 4);
        assert_eq!(pl.total_duration_ms, 240_000);
        // repetir uma existente e uma repetida na própria lista
        let e = add_track(&db, "E", "Art", 30_000);
        let pl = db
            .add_tracks_to_playlist(&pl.id, &[ts[0].id.clone(), e.id.clone(), e.id.clone()])
            .unwrap();
        assert_eq!(pl.track_count, 5);
        assert_eq!(pl.total_duration_ms, 270_000);
        assert_eq!(
            positions(&db, &pl.id).last().unwrap(),
            &("E".to_string(), 4)
        );
        // faixa inexistente aborta tudo
        let r = db.add_tracks_to_playlist(&pl.id, &[ts[1].id.clone(), "nope".into()]);
        assert_eq!(r.unwrap_err().code(), "not_found");
        assert_eq!(db.get_playlist(&pl.id).unwrap().track_count, 5);
        assert_eq!(
            db.add_tracks_to_playlist("nope", &[]).unwrap_err().code(),
            "not_found"
        );
    }

    // @test RF-07
    #[test]
    fn mover_faixa_mantem_posicoes_contiguas() {
        let db = db();
        let (pl, ts) = playlist_abcd(&db);
        let names =
            |db: &Database| titles(&db.playlist_tracks(&pl.id, &TrackQuery::default()).unwrap());
        assert_eq!(names(&db), ["A", "B", "C", "D"]);

        db.move_playlist_track(&pl.id, &ts[3].id, 0).unwrap(); // D para o início
        assert_eq!(names(&db), ["D", "A", "B", "C"]);
        db.move_playlist_track(&pl.id, &ts[3].id, 2).unwrap(); // D para o meio
        assert_eq!(names(&db), ["A", "B", "D", "C"]);
        db.move_playlist_track(&pl.id, &ts[0].id, 99).unwrap(); // além do fim: vai pro fim
        assert_eq!(names(&db), ["B", "D", "C", "A"]);
        db.move_playlist_track(&pl.id, &ts[0].id, 3).unwrap(); // já está lá: sem mudança
        assert_eq!(names(&db), ["B", "D", "C", "A"]);
        let pos: Vec<i64> = positions(&db, &pl.id).into_iter().map(|p| p.1).collect();
        assert_eq!(pos, [0, 1, 2, 3]);

        assert_eq!(
            db.move_playlist_track(&pl.id, &ts[0].id, -1)
                .unwrap_err()
                .code(),
            "invalid_input"
        );
        let fora = add_track(&db, "Fora", "X", 1);
        assert_eq!(
            db.move_playlist_track(&pl.id, &fora.id, 0)
                .unwrap_err()
                .code(),
            "not_found"
        );
    }

    // @test RF-07
    #[test]
    fn remover_recompacta_e_nova_adicao_vai_pro_fim() {
        let db = db();
        let (pl, ts) = playlist_abcd(&db);
        let pl = db
            .remove_tracks_from_playlist(&pl.id, &[ts[1].id.clone(), ts[2].id.clone(), "x".into()])
            .unwrap();
        assert_eq!(pl.track_count, 2);
        assert_eq!(positions(&db, &pl.id), [("A".into(), 0), ("D".into(), 1)]);
        db.add_tracks_to_playlist(&pl.id, &[ts[1].id.clone()])
            .unwrap();
        assert_eq!(
            positions(&db, &pl.id),
            [("A".into(), 0), ("D".into(), 1), ("B".into(), 2)]
        );
        // as faixas continuam na biblioteca
        assert_eq!(db.list_tracks(&TrackQuery::default()).unwrap().len(), 4);
    }

    // @test RF-07
    #[test]
    fn playlist_ordena_por_posicao_titulo_e_data() {
        let db = db();
        let (pl, ts) = playlist_abcd(&db);
        db.move_playlist_track(&pl.id, &ts[3].id, 0).unwrap(); // D A B C
        let q = |by, dir| TrackQuery {
            sort_by: Some(by),
            sort_dir: dir,
            ..Default::default()
        };
        let get = |query: TrackQuery| titles(&db.playlist_tracks(&pl.id, &query).unwrap());
        assert_eq!(get(q(TrackSort::Position, None)), ["D", "A", "B", "C"]);
        assert_eq!(
            get(q(TrackSort::Position, Some(SortDir::Desc))),
            ["C", "B", "A", "D"]
        );
        assert_eq!(
            get(q(TrackSort::Title, Some(SortDir::Desc))),
            ["D", "C", "B", "A"]
        );
        assert_eq!(get(TrackQuery::default()), ["D", "A", "B", "C"]);
    }

    // @test RF-08
    #[test]
    fn busca_e_favoritos_dentro_da_playlist() {
        let db = db();
        let (pl, ts) = playlist_abcd(&db);
        let fora = add_track(&db, "Árvore Fora", "Art", 1);
        db.set_favorite(&ts[2].id, true).unwrap();
        let s = TrackQuery {
            search: Some("arvore".into()),
            ..Default::default()
        };
        assert!(
            db.playlist_tracks(&pl.id, &s).unwrap().is_empty(),
            "faixa de fora não aparece"
        );
        db.add_tracks_to_playlist(&pl.id, std::slice::from_ref(&fora.id))
            .unwrap();
        assert_eq!(
            titles(&db.playlist_tracks(&pl.id, &s).unwrap()),
            ["Árvore Fora"]
        );
        let f = TrackQuery {
            favorites_only: Some(true),
            ..Default::default()
        };
        assert_eq!(titles(&db.playlist_tracks(&pl.id, &f).unwrap()), ["C"]);
        assert_eq!(
            db.playlist_tracks("nope", &s).unwrap_err().code(),
            "not_found"
        );
    }

    // @test RF-06
    #[test]
    fn apagar_playlist_leva_os_itens_mas_nao_as_faixas() {
        let db = db();
        let (pl, _) = playlist_abcd(&db);
        db.delete_playlist(&pl.id).unwrap();
        let n: i64 = db
            .conn
            .query_row("SELECT count(*) FROM playlist_tracks", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 0);
        assert_eq!(db.list_tracks(&TrackQuery::default()).unwrap().len(), 4);
    }

    // @test RF-22
    #[test]
    fn mosaico_ordena_artistas_e_prefere_foto_a_capa() {
        let db = db();
        let p = db.create_playlist("P", None).unwrap();
        let mut ids = Vec::new();
        for (artist, count) in [("Bia", 2), ("Ana", 2), ("Cris", 1), ("Duda", 1), ("Eva", 1)] {
            for n in 0..count {
                let t = add_track(&db, &format!("{artist} {n}"), artist, 1000);
                db.update_track(
                    &t.id,
                    &crate::TrackPatch {
                        cover_path: Some(Some(format!("/{artist}-{n}.jpg"))),
                        ..Default::default()
                    },
                )
                .unwrap();
                ids.push(t.id);
            }
        }
        db.add_tracks_to_playlist(&p.id, &ids).unwrap();
        let bia = db.get_or_create_artist("Bia").unwrap();
        db.set_artist_image(&bia.id, Some("/bia-artista.jpg"), Some("crédito"))
            .unwrap();
        assert_eq!(
            db.get_playlist(&p.id).unwrap().mosaic_paths,
            [
                "/Ana-0.jpg",
                "/bia-artista.jpg",
                "/Cris-0.jpg",
                "/Duda-0.jpg"
            ]
        );
        assert_eq!(
            db.set_playlist_cover(&p.id, Some("/manual.jpg")).unwrap(),
            None
        );
        assert_eq!(
            db.get_playlist(&p.id).unwrap().cover_path.as_deref(),
            Some("/manual.jpg")
        );
        assert_eq!(
            db.set_playlist_cover(&p.id, None).unwrap().as_deref(),
            Some("/manual.jpg")
        );
        assert_eq!(db.list_playlists().unwrap()[0].mosaic_paths.len(), 4);
    }

    // @test RF-22
    #[test]
    fn mosaico_nao_puxa_quinto_artista_quando_um_dos_quatro_nao_tem_imagem() {
        let db = db();
        let playlist = db.create_playlist("Mosaico", None).unwrap();
        let mut ids = Vec::new();
        for artist in ["Ana", "Bia", "Cris", "Duda"] {
            for n in 0..2 {
                let track = add_track(&db, &format!("{artist} {n}"), artist, 1000);
                if artist != "Ana" {
                    db.update_track(
                        &track.id,
                        &crate::TrackPatch {
                            cover_path: Some(Some(format!("/{artist}-{n}.jpg"))),
                            ..Default::default()
                        },
                    )
                    .unwrap();
                }
                ids.push(track.id);
            }
        }
        let fifth = add_track(&db, "Eva", "Eva", 1000);
        db.update_track(
            &fifth.id,
            &crate::TrackPatch {
                cover_path: Some(Some("/Eva.jpg".into())),
                ..Default::default()
            },
        )
        .unwrap();
        ids.push(fifth.id);
        db.add_tracks_to_playlist(&playlist.id, &ids).unwrap();
        assert_eq!(
            db.get_playlist(&playlist.id).unwrap().mosaic_paths,
            ["/Bia-0.jpg", "/Cris-0.jpg", "/Duda-0.jpg"]
        );
    }
}
