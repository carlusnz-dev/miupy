//! Artistas: get-or-create por nome, dados enriquecidos e listagem.

use rusqlite::{OptionalExtension, Row, params};

use crate::db::{Database, new_id};
use crate::error::{Error, Result};
use crate::model::{Artist, ArtistInfo, ArtistPatch};
use crate::text::{clean_opt, name_key};

const ARTIST_INFO_SQL: &str =
    "SELECT a.id, a.name, a.mbid, a.country, a.genres, a.bio, a.image_path, a.image_credit,
        (SELECT COUNT(*) FROM tracks t WHERE t.artist_id = a.id)
     FROM artists a";

fn genres_from_json(s: &str) -> Vec<String> {
    serde_json::from_str(s).unwrap_or_default()
}

fn info_from_row(r: &Row<'_>) -> rusqlite::Result<ArtistInfo> {
    Ok(ArtistInfo {
        id: r.get(0)?,
        name: r.get(1)?,
        mbid: r.get(2)?,
        country: r.get(3)?,
        genres: genres_from_json(&r.get::<_, String>(4)?),
        bio: r.get(5)?,
        image_path: r.get(6)?,
        image_credit: r.get(7)?,
        track_count: r.get(8)?,
    })
}

impl From<ArtistInfo> for Artist {
    fn from(i: ArtistInfo) -> Self {
        Artist {
            id: i.id,
            name: i.name,
            mbid: i.mbid,
            country: i.country,
            genres: i.genres,
            bio: i.bio,
            image_path: i.image_path,
        }
    }
}

impl Database {
    // @req RF-05
    /// Devolve o artista de nome `name` (sem diferenciar caixa, acento ou espaços repetidos),
    /// criando-o se não existir.
    pub fn get_or_create_artist(&self, name: &str) -> Result<Artist> {
        let display = name.split_whitespace().collect::<Vec<_>>().join(" ");
        if display.is_empty() {
            return Err(Error::invalid("o nome do artista não pode ser vazio"));
        }
        let key = name_key(&display);
        self.in_tx(|| {
            let existing: Option<String> = self
                .conn
                .query_row("SELECT id FROM artists WHERE name_key = ?1", [&key], |r| r.get(0))
                .optional()?;
            let id = match existing {
                Some(id) => id,
                None => {
                    let id = new_id();
                    self.conn.execute(
                        "INSERT INTO artists (id, name, name_key, created_at) VALUES (?1, ?2, ?3, ?4)",
                        params![id, display, key, self.now_ms()],
                    )?;
                    id
                }
            };
            Ok(self.get_artist_info(&id)?.into())
        })
    }

    // @req RF-12
    pub fn get_artist_info(&self, artist_id: &str) -> Result<ArtistInfo> {
        self.conn
            .query_row(
                &format!("{ARTIST_INFO_SQL} WHERE a.id = ?1"),
                [artist_id],
                info_from_row,
            )
            .optional()?
            .ok_or_else(|| Error::not_found("artista", artist_id))
    }

    // @req RF-12
    /// Artistas com ao menos uma faixa, por nome.
    pub fn list_artists(&self) -> Result<Vec<ArtistInfo>> {
        let sql = format!(
            "{ARTIST_INFO_SQL}
             WHERE EXISTS (SELECT 1 FROM tracks t WHERE t.artist_id = a.id)
             ORDER BY a.name COLLATE miupy_nocase, a.id"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map([], info_from_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    // @req RF-12
    /// Atualiza dados do artista (inclusive o nome, que reindexa a busca das faixas).
    pub fn update_artist(&self, artist_id: &str, patch: &ArtistPatch) -> Result<ArtistInfo> {
        self.in_tx(|| {
            let cur = self.get_artist_info(artist_id)?;
            let (name, key) = match &patch.name {
                Some(n) => {
                    let display = n.split_whitespace().collect::<Vec<_>>().join(" ");
                    if display.is_empty() {
                        return Err(Error::invalid("o nome do artista não pode ser vazio"));
                    }
                    let key = name_key(&display);
                    (display, key)
                }
                None => (cur.name.clone(), name_key(&cur.name)),
            };
            let pick = |new: &Option<Option<String>>, old: &Option<String>| match new {
                Some(v) => clean_opt(v.as_deref()),
                None => old.clone(),
            };
            let genres = patch.genres.clone().unwrap_or(cur.genres.clone());
            let genres_json = serde_json::to_string(&genres)
                .map_err(|e| Error::Internal(format!("gêneros: {e}")))?;
            self.conn.execute(
                "UPDATE artists SET name = ?2, name_key = ?3, mbid = ?4, country = ?5,
                        genres = ?6, bio = ?7, image_path = ?8,
                        image_credit = CASE WHEN ?9 THEN NULL ELSE image_credit END WHERE id = ?1",
                params![
                    artist_id,
                    name,
                    key,
                    pick(&patch.mbid, &cur.mbid),
                    pick(&patch.country, &cur.country),
                    genres_json,
                    pick(&patch.bio, &cur.bio),
                    pick(&patch.image_path, &cur.image_path),
                    patch.image_path.is_some(),
                ],
            )?;
            self.get_artist_info(artist_id)
        })
    }

    // @req RF-27
    /// Salva foto e crédito juntos e devolve o caminho anterior para a casca remover o arquivo.
    /// `None` em `image_path` remove também o crédito.
    pub fn set_artist_image(
        &self,
        artist_id: &str,
        image_path: Option<&str>,
        image_credit: Option<&str>,
    ) -> Result<Option<String>> {
        let old = self.get_artist_info(artist_id)?.image_path;
        let path = clean_opt(image_path);
        let credit = if path.is_some() {
            clean_opt(image_credit)
        } else {
            None
        };
        self.conn.execute(
            "UPDATE artists SET image_path = ?2, image_credit = ?3 WHERE id = ?1",
            params![artist_id, path, credit],
        )?;
        Ok(old)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::*;

    // @test RF-05
    #[test]
    fn get_or_create_reaproveita_por_nome_sem_caixa_nem_acento() {
        let db = db();
        let a = db.get_or_create_artist("  Sigur   Rós ").unwrap();
        assert_eq!(a.name, "Sigur Rós");
        let b = db.get_or_create_artist("SIGUR ROS").unwrap();
        assert_eq!(a.id, b.id);
        let c = db.get_or_create_artist("Outro").unwrap();
        assert_ne!(a.id, c.id);
        assert_eq!(
            db.get_or_create_artist("   ").unwrap_err().code(),
            "invalid_input"
        );
    }

    // @test RF-12
    #[test]
    fn atualiza_dados_enriquecidos_e_conta_faixas() {
        let db = db();
        let t = add_track(&db, "Hoppípolla", "Sigur Rós", 270_000);
        let id = t.artist_id.clone().unwrap();
        let info = db
            .update_artist(
                &id,
                &ArtistPatch {
                    mbid: Some(Some("abc".into())),
                    country: Some(Some("IS".into())),
                    genres: Some(vec!["post-rock".into(), "ambient".into()]),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(info.country.as_deref(), Some("IS"));
        assert_eq!(info.genres, vec!["post-rock", "ambient"]);
        assert_eq!(info.track_count, 1);
        // campo ausente mantém, null limpa
        let info = db
            .update_artist(
                &id,
                &ArtistPatch {
                    country: Some(None),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(info.country, None);
        assert_eq!(info.mbid.as_deref(), Some("abc"));
        assert_eq!(db.get_artist_info("nope").unwrap_err().code(), "not_found");
    }

    // @test RF-08
    #[test]
    fn renomear_artista_reindexa_a_busca() {
        let db = db();
        let t = add_track(&db, "Faixa", "Nome Velho", 1000);
        let id = t.artist_id.unwrap();
        db.update_artist(
            &id,
            &ArtistPatch {
                name: Some("Nome Novo".into()),
                ..Default::default()
            },
        )
        .unwrap();
        let q = |s: &str| crate::TrackQuery {
            search: Some(s.into()),
            ..Default::default()
        };
        assert_eq!(db.list_tracks(&q("novo")).unwrap().len(), 1);
        assert_eq!(db.list_tracks(&q("velho")).unwrap().len(), 0);
    }

    // @test RF-27
    #[test]
    fn foto_e_credito_do_artista_sao_atomicos() {
        let db = db();
        let artist = db.get_or_create_artist("Ana").unwrap();
        assert_eq!(
            db.set_artist_image(&artist.id, Some("/ana.jpg"), Some("Autora, CC BY-SA"))
                .unwrap(),
            None
        );
        let info = db.get_artist_info(&artist.id).unwrap();
        assert_eq!(info.image_credit.as_deref(), Some("Autora, CC BY-SA"));
        assert_eq!(
            db.set_artist_image(&artist.id, None, Some("crédito solto"))
                .unwrap()
                .as_deref(),
            Some("/ana.jpg")
        );
        let info = db.get_artist_info(&artist.id).unwrap();
        assert_eq!((info.image_path, info.image_credit), (None, None));
        assert_eq!(
            db.set_artist_image("missing", Some("/x"), None)
                .unwrap_err()
                .code(),
            "not_found"
        );
    }
}
