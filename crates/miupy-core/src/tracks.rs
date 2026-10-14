//! Faixas: criação/deduplicação, consulta com busca e ordenação, favoritos, metadados e
//! remoção.

use std::collections::HashSet;

use rusqlite::types::Value;
use rusqlite::{OptionalExtension, Row, params, params_from_iter};
use serde_json::json;

use crate::db::{Database, new_id};
use crate::error::{Error, Result};
use crate::model::{
    DeletedTrack, DownloadStatus, NewTrack, SortDir, Source, Track, TrackPatch, TrackQuery,
    TrackSort, Upserted,
};
use crate::text::{clean_opt, fts_query};

/// Colunas lidas por [`track_from_row`], nesta ordem.
pub(crate) const TRACK_COLUMNS: &str = "t.id, t.title, t.artist_id, a.name, t.album, \
     t.duration_ms, t.file_path, t.cover_path, t.source, t.source_id, t.codec, t.bitrate, \
     (f.track_id IS NOT NULL), t.download_status, t.added_at, t.genres, t.release_year";

pub(crate) const TRACK_FROM: &str = "tracks t \
     LEFT JOIN artists a ON a.id = t.artist_id \
     LEFT JOIN favorites f ON f.track_id = t.id";

pub(crate) fn track_from_row(r: &Row<'_>) -> rusqlite::Result<Track> {
    Ok(Track {
        id: r.get(0)?,
        title: r.get(1)?,
        artist_id: r.get(2)?,
        artist_name: r.get(3)?,
        album: r.get(4)?,
        duration_ms: r.get(5)?,
        file_path: r.get(6)?,
        cover_path: r.get(7)?,
        source: r.get(8)?,
        source_id: r.get(9)?,
        codec: r.get(10)?,
        bitrate: r.get(11)?,
        is_favorite: r.get(12)?,
        download_status: r.get(13)?,
        added_at: r.get(14)?,
        genres: serde_json::from_str(&r.get::<_, String>(15)?).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(15, rusqlite::types::Type::Text, Box::new(e))
        })?,
        release_year: r.get(16)?,
    })
}

// @req RF-20
pub(crate) fn normalized_genres(genres: &[String]) -> Vec<String> {
    let mut out: Vec<String> = genres
        .iter()
        .map(|g| g.trim().to_lowercase())
        .filter(|g| !g.is_empty())
        .collect();
    out.sort();
    out.dedup();
    out
}

fn valid_year(year: Option<i32>) -> Result<Option<i32>> {
    if year.is_some_and(|y| !(1..=9999).contains(&y)) {
        Err(Error::invalid(
            "ano de lançamento fora do intervalo 1..=9999",
        ))
    } else {
        Ok(year)
    }
}

fn require_title(title: &str) -> Result<String> {
    let t = title.trim();
    if t.is_empty() {
        return Err(Error::invalid("o título da faixa não pode ser vazio"));
    }
    Ok(t.to_owned())
}

impl Database {
    // @req RF-12 RF-20
    /// Lista as faixas para reprocessamento, em ordem de inclusão. `only_missing` seleciona
    /// ausência de gêneros, ano ou álbum; título e artista podem ser corrigidos via `TrackPatch`.
    pub fn tracks_for_metadata_refresh(&self, only_missing: bool) -> Result<Vec<Track>> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {TRACK_COLUMNS} FROM {TRACK_FROM}
             WHERE (?1 = 0 OR t.genres = '[]' OR t.release_year IS NULL OR t.album IS NULL)
             ORDER BY t.added_at, t.id"
        ))?;
        Ok(stmt
            .query_map([only_missing], track_from_row)?
            .collect::<rusqlite::Result<_>>()?)
    }
    // @req RF-05
    pub fn get_track(&self, track_id: &str) -> Result<Track> {
        self.conn
            .query_row(
                &format!("SELECT {TRACK_COLUMNS} FROM {TRACK_FROM} WHERE t.id = ?1"),
                [track_id],
                track_from_row,
            )
            .optional()?
            .ok_or_else(|| Error::not_found("faixa", track_id))
    }

    // @req RF-08 RF-07
    /// Biblioteca inteira, filtrada e ordenada por `query`. Sem `sortBy`: mais recentes
    /// primeiro. `sortBy = position` não tem sentido fora de uma playlist e vira `addedAt`.
    pub fn list_tracks(&self, query: &TrackQuery) -> Result<Vec<Track>> {
        self.select_tracks(None, None, query)
    }

    // @req RF-05 RF-08
    /// Faixas do artista com a mesma busca, ordenação e filtro de favoritas da biblioteca.
    pub fn list_tracks_by_artist(&self, artist_id: &str, query: &TrackQuery) -> Result<Vec<Track>> {
        self.get_artist_info(artist_id)?;
        self.select_tracks(None, Some(artist_id), query)
    }

    // @req RF-03
    /// Resolve a fila na ordem recebida; IDs inexistentes são ignorados.
    pub fn get_tracks_by_ids(&self, track_ids: &[String]) -> Result<Vec<Track>> {
        let mut out = Vec::with_capacity(track_ids.len());
        for id in track_ids {
            match self.get_track(id) {
                Ok(track) => out.push(track),
                Err(Error::NotFound(_)) => {}
                Err(e) => return Err(e),
            }
        }
        Ok(out)
    }

    /// Consulta comum à biblioteca (`playlist_id = None`) e às playlists.
    pub(crate) fn select_tracks(
        &self,
        playlist_id: Option<&str>,
        artist_id: Option<&str>,
        q: &TrackQuery,
    ) -> Result<Vec<Track>> {
        let mut sql = format!("SELECT {TRACK_COLUMNS} FROM {TRACK_FROM}");
        let mut args: Vec<Value> = Vec::new();
        let mut wheres: Vec<&str> = Vec::new();

        if let Some(pid) = playlist_id {
            sql.push_str(" JOIN playlist_tracks pt ON pt.track_id = t.id AND pt.playlist_id = ?");
            args.push(pid.to_owned().into());
        }
        if let Some(aid) = artist_id {
            wheres.push("t.artist_id = ?");
            args.push(aid.to_owned().into());
        }
        if let Some(text) = q.search.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            // só pontuação: nada pesquisável, nenhum resultado
            let Some(fts) = fts_query(text) else {
                return Ok(Vec::new());
            };
            wheres.push("t.rowid IN (SELECT rowid FROM tracks_fts WHERE tracks_fts MATCH ?)");
            args.push(fts.into());
        }
        if q.favorites_only == Some(true) {
            wheres.push("f.track_id IS NOT NULL");
        }
        if !wheres.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&wheres.join(" AND "));
        }

        let dir = if q.sort_dir == Some(SortDir::Desc) {
            "DESC"
        } else {
            "ASC"
        };
        let order = match (q.sort_by, playlist_id.is_some()) {
            (None, true) => "pt.position ASC, t.id".to_owned(),
            (None, false) => "t.added_at DESC, t.id DESC".to_owned(),
            (Some(TrackSort::Position), true) => format!("pt.position {dir}, t.id"),
            (Some(TrackSort::Position), false) | (Some(TrackSort::AddedAt), _) => {
                format!("t.added_at {dir}, t.id {dir}")
            }
            (Some(TrackSort::Title), _) => {
                format!("t.title COLLATE miupy_nocase {dir}, t.id")
            }
            (Some(TrackSort::Artist), _) => format!(
                "COALESCE(a.name, '') COLLATE miupy_nocase {dir}, \
                 t.title COLLATE miupy_nocase, t.id"
            ),
            (Some(TrackSort::Album), _) => format!(
                "COALESCE(t.album, '') COLLATE miupy_nocase {dir}, \
                 t.title COLLATE miupy_nocase, t.id"
            ),
            (Some(TrackSort::Duration), _) => {
                format!("t.duration_ms {dir}, t.title COLLATE miupy_nocase, t.id")
            }
        };
        sql.push_str(" ORDER BY ");
        sql.push_str(&order);

        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(args), track_from_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    // @req RF-09 RF-05
    /// Cria a faixa ou devolve a existente. Dedupe: `(source, source_id)` quando há
    /// `source_id`; senão `file_path`. Numa faixa existente só preenche lacunas
    /// (miniatura, duração zerada): nunca sobrescreve o que o usuário editou.
    pub fn upsert_track(&self, new: &NewTrack) -> Result<Upserted> {
        let title = require_title(&new.title)?;
        let genres = serde_json::to_string(&normalized_genres(&new.genres))
            .map_err(|e| Error::Internal(e.to_string()))?;
        let release_year = valid_year(new.release_year)?;
        if new.duration_ms < 0 {
            return Err(Error::invalid("a duração não pode ser negativa"));
        }
        let source_id = clean_opt(new.source_id.as_deref());
        let file_path = clean_opt(new.file_path.as_deref());
        match new.source {
            Source::Youtube if source_id.is_none() => {
                return Err(Error::invalid("faixa do YouTube exige source_id"));
            }
            Source::Local if file_path.is_none() => {
                return Err(Error::invalid("faixa local exige file_path"));
            }
            _ => {}
        }

        self.in_tx(|| {
            let existing: Option<String> = if let Some(sid) = &source_id {
                self.conn
                    .query_row(
                        "SELECT id FROM tracks WHERE source = ?1 AND source_id = ?2",
                        params![new.source, sid],
                        |r| r.get(0),
                    )
                    .optional()?
            } else if let Some(fp) = &file_path {
                self.conn
                    .query_row("SELECT id FROM tracks WHERE file_path = ?1", [fp], |r| {
                        r.get(0)
                    })
                    .optional()?
            } else {
                None
            };

            if let Some(id) = existing {
                self.conn.execute(
                    "UPDATE tracks SET
                        thumbnail_url = COALESCE(thumbnail_url, ?2),
                        duration_ms = CASE WHEN duration_ms = 0 THEN ?3 ELSE duration_ms END,
                        genres = CASE WHEN genres = '[]' THEN ?4 ELSE genres END,
                        release_year = COALESCE(release_year, ?5),
                        mbid = COALESCE(mbid, ?6), isrc = COALESCE(isrc, ?7)
                     WHERE id = ?1",
                    params![id, clean_opt(new.thumbnail_url.as_deref()), new.duration_ms,
                        genres, release_year, clean_opt(new.mbid.as_deref()), clean_opt(new.isrc.as_deref())],
                )?;
                return Ok(Upserted {
                    track: self.get_track(&id)?,
                    created: false,
                });
            }

            let artist_id = match clean_opt(new.artist_name.as_deref()) {
                Some(name) => Some(self.get_or_create_artist(&name)?.id),
                None => None,
            };
            let id = new_id();
            let status = if file_path.is_some() {
                DownloadStatus::Done
            } else {
                DownloadStatus::None
            };
            self.conn.execute(
                "INSERT INTO tracks (id, title, artist_id, album, duration_ms, file_path,
                        cover_path, source, source_id, codec, bitrate, thumbnail_url,
                        download_status, added_at, genres, release_year, mbid, isrc)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
                params![
                    id,
                    title,
                    artist_id,
                    clean_opt(new.album.as_deref()),
                    new.duration_ms,
                    file_path,
                    clean_opt(new.cover_path.as_deref()),
                    new.source,
                    source_id,
                    clean_opt(new.codec.as_deref()),
                    new.bitrate,
                    clean_opt(new.thumbnail_url.as_deref()),
                    status,
                    self.now_ms(),
                    genres,
                    release_year,
                    clean_opt(new.mbid.as_deref()),
                    clean_opt(new.isrc.as_deref()),
                ],
            )?;
            self.record_action(
                "track_added",
                Some("track"),
                Some(&id),
                Some(&json!({ "source": new.source.as_str() })),
            )?;
            Ok(Upserted {
                track: self.get_track(&id)?,
                created: true,
            })
        })
    }

    // @req RF-09
    /// Dos `source_ids` dados, quais já estão na biblioteca (para `SearchResult.inLibrary`).
    pub fn known_source_ids(
        &self,
        source: Source,
        source_ids: &[String],
    ) -> Result<HashSet<String>> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT 1 FROM tracks WHERE source = ?1 AND source_id = ?2")?;
        let mut found = HashSet::new();
        for sid in source_ids {
            if stmt.exists(params![source, sid])? {
                found.insert(sid.clone());
            }
        }
        Ok(found)
    }

    // @req RF-10
    /// Miniatura remota registrada na criação da faixa (para o downloader baixar a capa).
    pub fn track_thumbnail_url(&self, track_id: &str) -> Result<Option<String>> {
        self.conn
            .query_row(
                "SELECT thumbnail_url FROM tracks WHERE id = ?1",
                [track_id],
                |r| r.get(0),
            )
            .optional()?
            .ok_or_else(|| Error::not_found("faixa", track_id))
    }

    // @req RF-04
    /// Favorita ou desfavorita. Idempotente.
    pub fn set_favorite(&self, track_id: &str, favorite: bool) -> Result<Track> {
        self.in_tx(|| {
            self.require_track(track_id)?;
            let changed = if favorite {
                self.conn.execute(
                    "INSERT OR IGNORE INTO favorites (track_id, created_at) VALUES (?1, ?2)",
                    params![track_id, self.now_ms()],
                )?
            } else {
                self.conn
                    .execute("DELETE FROM favorites WHERE track_id = ?1", [track_id])?
            };
            if changed > 0 {
                let kind = if favorite {
                    "track_favorited"
                } else {
                    "track_unfavorited"
                };
                self.record_action(kind, Some("track"), Some(track_id), None)?;
            }
            self.get_track(track_id)
        })
    }

    // @req RF-05
    /// Altera metadados (título, artista, álbum, duração, capa). Trocar o artista faz
    /// get-or-create pelo nome; o artista antigo permanece no banco (preserva o enriquecimento).
    pub fn update_track(&self, track_id: &str, patch: &TrackPatch) -> Result<Track> {
        self.in_tx(|| {
            let cur = self.get_track(track_id)?;
            let title = match &patch.title {
                Some(t) => require_title(t)?,
                None => cur.title.clone(),
            };
            let artist_id = match &patch.artist_name {
                Some(Some(name)) => match clean_opt(Some(name)) {
                    Some(n) => Some(self.get_or_create_artist(&n)?.id),
                    None => None,
                },
                Some(None) => None,
                None => cur.artist_id.clone(),
            };
            let album = match &patch.album {
                Some(v) => clean_opt(v.as_deref()),
                None => cur.album.clone(),
            };
            let cover = match &patch.cover_path {
                Some(v) => clean_opt(v.as_deref()),
                None => cur.cover_path.clone(),
            };
            let duration = patch.duration_ms.unwrap_or(cur.duration_ms);
            let genres = match &patch.genres {
                Some(v) => serde_json::to_string(&normalized_genres(v))
                    .map_err(|e| Error::Internal(e.to_string()))?,
                None => serde_json::to_string(&cur.genres)
                    .map_err(|e| Error::Internal(e.to_string()))?,
            };
            let release_year = valid_year(patch.release_year.unwrap_or(cur.release_year))?;
            if duration < 0 {
                return Err(Error::invalid("a duração não pode ser negativa"));
            }
            self.conn.execute(
                "UPDATE tracks SET title = ?2, artist_id = ?3, album = ?4, duration_ms = ?5,
                        cover_path = ?6, genres = ?7, release_year = ?8,
                        mbid = CASE WHEN ?9 THEN ?10 ELSE mbid END,
                        isrc = CASE WHEN ?11 THEN ?12 ELSE isrc END WHERE id = ?1",
                params![
                    track_id,
                    title,
                    artist_id,
                    album,
                    duration,
                    cover,
                    genres,
                    release_year,
                    patch.mbid.is_some(),
                    patch.mbid.as_ref().and_then(|v| v.as_deref()),
                    patch.isrc.is_some(),
                    patch.isrc.as_ref().and_then(|v| v.as_deref())
                ],
            )?;
            self.get_track(track_id)
        })
    }

    // @req RF-06
    /// Apaga a faixa. Cascata: favorito, itens de playlist (as posições são recompactadas),
    /// letra, job de download e histórico de reprodução. Devolve os caminhos para o chamador
    /// remover os arquivos, se quiser (`deleteFile` do ipc).
    pub fn delete_track(&self, track_id: &str) -> Result<DeletedTrack> {
        self.in_tx(|| {
            let (file_path, cover_path): (Option<String>, Option<String>) = self
                .conn
                .query_row(
                    "SELECT file_path, cover_path FROM tracks WHERE id = ?1",
                    [track_id],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()?
                .ok_or_else(|| Error::not_found("faixa", track_id))?;

            let playlists: Vec<String> = {
                let mut stmt = self
                    .conn
                    .prepare("SELECT playlist_id FROM playlist_tracks WHERE track_id = ?1")?;
                stmt.query_map([track_id], |r| r.get(0))?
                    .collect::<rusqlite::Result<_>>()?
            };
            self.conn
                .execute("DELETE FROM tracks WHERE id = ?1", [track_id])?;
            for pid in &playlists {
                self.renumber_playlist(pid)?;
                self.touch_playlist(pid)?;
            }
            self.record_action("track_removed", Some("track"), Some(track_id), None)?;
            Ok(DeletedTrack {
                track_id: track_id.to_owned(),
                file_path,
                cover_path,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::*;

    // @test RF-05 RF-08 RF-03 RF-20
    #[test]
    fn metadados_artista_e_fila_preservam_contrato() {
        let db = db();
        let mut n = NewTrack::youtube("metadata", "Árvore", Some("Ana".into()), 30_000, None);
        n.genres = vec![" ROCK ".into(), "rock".into(), "Jazz".into()];
        n.release_year = Some(2020);
        n.mbid = Some("mbid-1".into());
        n.isrc = Some("isrc-1".into());
        let a = db.upsert_track(&n).unwrap().track;
        assert_eq!(a.genres, ["jazz", "rock"]);
        assert_eq!(a.release_year, Some(2020));
        let serialized = serde_json::to_value(&a).unwrap();
        assert_eq!(serialized["releaseYear"], 2020);
        assert_eq!(serialized["genres"], serde_json::json!(["jazz", "rock"]));
        let b = add_track(&db, "Outra", "Ana", 10_000);
        let c = add_track(&db, "Fora", "Bia", 10_000);
        db.set_favorite(&a.id, true).unwrap();
        db.set_favorite(&c.id, true).unwrap();
        let q = TrackQuery {
            search: Some("arv".into()),
            favorites_only: Some(true),
            sort_by: Some(TrackSort::Title),
            ..Default::default()
        };
        assert_eq!(
            titles(
                &db.list_tracks_by_artist(a.artist_id.as_deref().unwrap(), &q)
                    .unwrap()
            ),
            ["Árvore"]
        );
        assert_eq!(
            db.list_tracks_by_artist("nope", &q).unwrap_err().code(),
            "not_found"
        );
        db.delete_track(&c.id).unwrap();
        assert_eq!(
            titles(
                &db.get_tracks_by_ids(&[b.id.clone(), c.id.clone(), a.id.clone(), b.id.clone()])
                    .unwrap()
            ),
            ["Outra", "Árvore", "Outra"]
        );
        let patched = db
            .update_track(
                &a.id,
                &TrackPatch {
                    genres: Some(vec!["POP".into(), "pop".into()]),
                    release_year: Some(None),
                    mbid: Some(None),
                    isrc: Some(None),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(patched.genres, ["pop"]);
        assert_eq!(patched.release_year, None);
        let meta: (Option<String>, Option<String>) = db
            .conn
            .query_row(
                "SELECT mbid, isrc FROM tracks WHERE id = ?1",
                [&a.id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(meta, (None, None));
    }

    fn q(search: &str) -> TrackQuery {
        TrackQuery {
            search: Some(search.into()),
            ..Default::default()
        }
    }

    // @test RF-09
    #[test]
    fn upsert_do_youtube_deduplica_por_source_id() {
        let db = db();
        let n = NewTrack::youtube("abc123", "Música", Some("Banda".into()), 0, None);
        let a = db.upsert_track(&n).unwrap();
        assert!(a.created);
        assert_eq!(a.track.download_status, DownloadStatus::None);
        assert_eq!(a.track.file_path, None);
        assert_eq!(a.track.artist_name.as_deref(), Some("Banda"));

        // mesma origem: devolve a existente, só preenche lacunas
        let n2 = NewTrack::youtube(
            "abc123",
            "Título diferente",
            Some("Outra".into()),
            200_000,
            Some("http://img".into()),
        );
        let b = db.upsert_track(&n2).unwrap();
        assert!(!b.created);
        assert_eq!(b.track.id, a.track.id);
        assert_eq!(b.track.title, "Música", "não sobrescreve o título");
        assert_eq!(b.track.duration_ms, 200_000, "preenche duração zerada");
        assert_eq!(
            db.track_thumbnail_url(&a.track.id).unwrap().as_deref(),
            Some("http://img")
        );
        assert_eq!(db.list_tracks(&TrackQuery::default()).unwrap().len(), 1);

        let known = db
            .known_source_ids(Source::Youtube, &["abc123".into(), "zzz".into()])
            .unwrap();
        assert_eq!(known, HashSet::from(["abc123".to_string()]));
    }

    // @test RF-09
    #[test]
    fn upsert_valida_entrada() {
        let db = db();
        let vazio = NewTrack::youtube("x", "   ", None, 0, None);
        assert_eq!(db.upsert_track(&vazio).unwrap_err().code(), "invalid_input");
        let sem_id = NewTrack {
            source_id: None,
            ..NewTrack::youtube("x", "t", None, 0, None)
        };
        assert_eq!(
            db.upsert_track(&sem_id).unwrap_err().code(),
            "invalid_input"
        );
        let neg = NewTrack::youtube("x", "t", None, -1, None);
        assert_eq!(db.upsert_track(&neg).unwrap_err().code(), "invalid_input");
    }

    // @test RF-05
    #[test]
    fn faixa_local_ja_nasce_baixada_e_deduplica_por_arquivo() {
        let db = db();
        let a = db.upsert_track(&NewTrack::local("/m/a.m4a", "A")).unwrap();
        assert_eq!(a.track.download_status, DownloadStatus::Done);
        assert_eq!(a.track.source, Source::Local);
        let b = db
            .upsert_track(&NewTrack::local("/m/a.m4a", "A de novo"))
            .unwrap();
        assert!(!b.created);
        assert_eq!(b.track.id, a.track.id);
    }

    // @test RF-04
    #[test]
    fn favorita_e_desfavorita_com_filtro() {
        let db = db();
        let a = add_track(&db, "A", "X", 1000);
        let b = add_track(&db, "B", "X", 1000);
        assert!(!a.is_favorite);
        assert!(db.set_favorite(&a.id, true).unwrap().is_favorite);
        db.set_favorite(&a.id, true).unwrap(); // idempotente
        let only = TrackQuery {
            favorites_only: Some(true),
            ..Default::default()
        };
        let favs = db.list_tracks(&only).unwrap();
        assert_eq!(favs.len(), 1);
        assert_eq!(favs[0].id, a.id);
        assert!(!db.set_favorite(&a.id, false).unwrap().is_favorite);
        assert!(db.list_tracks(&only).unwrap().is_empty());
        assert_eq!(
            db.set_favorite("nope", true).unwrap_err().code(),
            "not_found"
        );
        let _ = b;
        // só uma ação por mudança real
        let acoes = db.list_actions(Some("track_favorited"), 10).unwrap();
        assert_eq!(acoes.len(), 1);
    }

    // @test RF-08
    #[test]
    fn busca_ignora_acento_caixa_e_aceita_prefixo() {
        let db = db();
        add_track_album(&db, "Não Vá Embora", "Marisa Monte", "Memórias", 1000);
        add_track_album(&db, "Árvore", "Zeca", "Floresta", 1000);
        add_track_album(&db, "Outra", "Beyoncé", "Lemonade", 1000);

        assert_eq!(
            titles(&db.list_tracks(&q("nao va")).unwrap()),
            ["Não Vá Embora"]
        );
        assert_eq!(titles(&db.list_tracks(&q("ARVORE")).unwrap()), ["Árvore"]);
        assert_eq!(titles(&db.list_tracks(&q("beyonce")).unwrap()), ["Outra"]);
        assert_eq!(
            titles(&db.list_tracks(&q("memorias")).unwrap()),
            ["Não Vá Embora"]
        );
        assert_eq!(titles(&db.list_tracks(&q("Árv")).unwrap()), ["Árvore"]);
        assert_eq!(
            db.list_tracks(&q("zeca flor")).unwrap().len(),
            1,
            "palavras em AND"
        );
        assert!(db.list_tracks(&q("inexistente")).unwrap().is_empty());
        // entrada hostil não quebra a sintaxe do FTS
        assert!(db.list_tracks(&q("\"* OR (")).unwrap().is_empty());
        assert!(db.list_tracks(&q("!!!")).unwrap().is_empty());
        assert_eq!(
            db.list_tracks(&q("   ")).unwrap().len(),
            3,
            "busca em branco não filtra"
        );
    }

    // @test RF-08
    #[test]
    fn busca_acompanha_edicao_e_remocao() {
        let db = db();
        let t = add_track(&db, "Antes", "Banda", 1000);
        db.update_track(
            &t.id,
            &TrackPatch {
                title: Some("Depois".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(db.list_tracks(&q("antes")).unwrap().is_empty());
        assert_eq!(db.list_tracks(&q("depois")).unwrap().len(), 1);
        db.delete_track(&t.id).unwrap();
        assert!(db.list_tracks(&q("depois")).unwrap().is_empty());
    }

    // @test RF-07
    #[test]
    fn ordenacao_por_cada_campo() {
        let db = db();
        add_track_album_at(&db, "banana", "Zé", "Alfa", 300, 10);
        add_track_album_at(&db, "Árvore", "ana", "Beta", 100, 30);
        add_track_album_at(&db, "Cereja", "Bia", "Gama", 200, 20);
        let sorted = |by, dir| {
            let q = TrackQuery {
                sort_by: Some(by),
                sort_dir: dir,
                ..Default::default()
            };
            titles(&db.list_tracks(&q).unwrap())
        };
        // Árvore vem antes de banana: a ordenação ignora acento e caixa
        assert_eq!(
            sorted(TrackSort::Title, None),
            ["Árvore", "banana", "Cereja"]
        );
        assert_eq!(
            sorted(TrackSort::Title, Some(SortDir::Desc)),
            ["Cereja", "banana", "Árvore"]
        );
        assert_eq!(
            sorted(TrackSort::Artist, None),
            ["Árvore", "Cereja", "banana"]
        );
        assert_eq!(
            sorted(TrackSort::Album, None),
            ["banana", "Árvore", "Cereja"]
        );
        assert_eq!(
            sorted(TrackSort::Duration, None),
            ["Árvore", "Cereja", "banana"]
        );
        assert_eq!(
            sorted(TrackSort::AddedAt, None),
            ["banana", "Cereja", "Árvore"]
        );
        assert_eq!(
            sorted(TrackSort::AddedAt, Some(SortDir::Desc)),
            ["Árvore", "Cereja", "banana"]
        );
        // sem sortBy: mais recentes primeiro; position na biblioteca vira addedAt
        assert_eq!(
            titles(&db.list_tracks(&TrackQuery::default()).unwrap()),
            ["Árvore", "Cereja", "banana"]
        );
        assert_eq!(
            sorted(TrackSort::Position, None),
            ["banana", "Cereja", "Árvore"]
        );
    }

    // @test RF-05
    #[test]
    fn atualiza_metadados_e_troca_de_artista() {
        let db = db();
        let t = add_track(&db, "Velho", "Antigo", 1000);
        let u = db
            .update_track(
                &t.id,
                &TrackPatch {
                    title: Some("Novo".into()),
                    artist_name: Some(Some("Novo Artista".into())),
                    album: Some(Some("Disco".into())),
                    duration_ms: Some(5000),
                    cover_path: Some(Some("/c.jpg".into())),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(u.title, "Novo");
        assert_eq!(u.artist_name.as_deref(), Some("Novo Artista"));
        assert_eq!(u.album.as_deref(), Some("Disco"));
        assert_eq!(u.duration_ms, 5000);
        assert_eq!(u.cover_path.as_deref(), Some("/c.jpg"));
        // tirar o artista e o álbum
        let u = db
            .update_track(
                &t.id,
                &TrackPatch {
                    artist_name: Some(None),
                    album: Some(None),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!((u.artist_id, u.album), (None, None));
        assert_eq!(
            db.list_artists().unwrap().len(),
            0,
            "artista sem faixas some da listagem"
        );
        let bad = TrackPatch {
            title: Some("  ".into()),
            ..Default::default()
        };
        assert_eq!(
            db.update_track(&t.id, &bad).unwrap_err().code(),
            "invalid_input"
        );
        assert_eq!(
            db.update_track("nope", &TrackPatch::default())
                .unwrap_err()
                .code(),
            "not_found"
        );
    }

    // @test RF-06
    #[test]
    fn apagar_faixa_em_cascata_e_devolve_caminhos() {
        let db = db();
        let t = db
            .upsert_track(&NewTrack::local("/m/a.m4a", "A"))
            .unwrap()
            .track;
        let keep = add_track(&db, "B", "X", 1000);
        let pl = db.create_playlist("P", None).unwrap();
        db.add_tracks_to_playlist(&pl.id, &[keep.id.clone(), t.id.clone()])
            .unwrap();
        db.set_favorite(&t.id, true).unwrap();
        db.set_lyrics(&crate::Lyrics {
            track_id: t.id.clone(),
            plain: Some("x".into()),
            synced: None,
        })
        .unwrap();
        let ev = db.start_play(&t.id, Some(&pl.id)).unwrap();
        db.add_listened(&ev, 1000).unwrap();

        let d = db.delete_track(&t.id).unwrap();
        assert_eq!(d.file_path.as_deref(), Some("/m/a.m4a"));
        assert_eq!(db.get_track(&t.id).unwrap_err().code(), "not_found");
        assert_eq!(db.delete_track(&t.id).unwrap_err().code(), "not_found");
        let n: i64 = db
            .conn
            .query_row("SELECT count(*) FROM favorites", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 0);
        for tabela in ["play_events", "lyrics", "tracks_fts"] {
            let sql = format!("SELECT count(*) FROM {tabela}");
            let n: i64 = db.conn.query_row(&sql, [], |r| r.get(0)).unwrap();
            assert_eq!(n, if tabela == "tracks_fts" { 1 } else { 0 }, "{tabela}");
        }
        // a playlist ficou com a faixa restante, posição recompactada
        let restantes = db.playlist_tracks(&pl.id, &TrackQuery::default()).unwrap();
        assert_eq!(titles(&restantes), ["B"]);
        assert_eq!(db.get_playlist(&pl.id).unwrap().track_count, 1);
    }

    // @test RF-12 RF-20
    #[test]
    fn lista_para_reprocessamento_aplica_ou_nas_lacunas() {
        let db = db();
        let mut complete = NewTrack::youtube("complete", "C", Some("Ana".into()), 1000, None);
        complete.genres = vec!["rock".into()];
        complete.release_year = Some(2020);
        complete.album = Some("Disco".into());
        let complete = db.upsert_track(&complete).unwrap().track;
        let genre = add_track_album(&db, "G", "Ana", "Disco", 1000);
        let year = add_track_album(&db, "Y", "Ana", "Disco", 1000);
        db.update_track(
            &year.id,
            &TrackPatch {
                genres: Some(vec!["rock".into()]),
                ..Default::default()
            },
        )
        .unwrap();
        let album = add_track(&db, "A", "Ana", 1000);
        db.update_track(
            &album.id,
            &TrackPatch {
                genres: Some(vec!["rock".into()]),
                release_year: Some(Some(2020)),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(db.tracks_for_metadata_refresh(false).unwrap().len(), 4);
        let missing = db.tracks_for_metadata_refresh(true).unwrap();
        assert_eq!(missing.len(), 3);
        assert!(!missing.iter().any(|t| t.id == complete.id));
        assert!(missing.iter().any(|t| t.id == genre.id));
        assert!(missing.iter().any(|t| t.id == year.id));
        assert!(missing.iter().any(|t| t.id == album.id));
        let fixed = db
            .update_track(
                &album.id,
                &TrackPatch {
                    title: Some("Corrigida".into()),
                    artist_name: Some(Some("Bia".into())),
                    album: Some(Some("Novo".into())),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(
            (
                fixed.title.as_str(),
                fixed.artist_name.as_deref(),
                fixed.album.as_deref()
            ),
            ("Corrigida", Some("Bia"), Some("Novo"))
        );
    }
}
