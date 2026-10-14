//! Letras (LRCLIB entra pela camada de mídia; aqui só persistência).

use rusqlite::{OptionalExtension, params};

use crate::db::Database;
use crate::error::{Error, Result};
use crate::model::Lyrics;
use crate::text::clean_opt;

impl Database {
    // @req RF-12
    /// Letra da faixa; sem registro devolve `plain` e `synced` nulos (a faixa precisa existir).
    pub fn get_lyrics(&self, track_id: &str) -> Result<Lyrics> {
        self.require_track(track_id)?;
        let row: Option<(Option<String>, Option<String>)> = self
            .conn
            .query_row(
                "SELECT plain, synced FROM lyrics WHERE track_id = ?1",
                [track_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let (plain, synced) = row.unwrap_or((None, None));
        Ok(Lyrics {
            track_id: track_id.to_owned(),
            plain,
            synced,
        })
    }

    // @req RF-12
    /// Grava (substituindo) a letra. Se `plain` e `synced` vierem vazios, remove o registro.
    pub fn set_lyrics(&self, lyrics: &Lyrics) -> Result<Lyrics> {
        self.require_track(&lyrics.track_id)?;
        let plain = clean_opt(lyrics.plain.as_deref());
        let synced = clean_opt(lyrics.synced.as_deref());
        if plain.is_none() && synced.is_none() {
            self.conn
                .execute("DELETE FROM lyrics WHERE track_id = ?1", [&lyrics.track_id])?;
        } else {
            self.conn.execute(
                "INSERT INTO lyrics (track_id, plain, synced, updated_at) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(track_id) DO UPDATE SET plain = ?2, synced = ?3, updated_at = ?4",
                params![lyrics.track_id, plain, synced, self.now_ms()],
            )?;
        }
        self.get_lyrics(&lyrics.track_id)
    }

    pub(crate) fn require_track(&self, track_id: &str) -> Result<()> {
        let exists: bool = self.conn.query_row(
            "SELECT EXISTS (SELECT 1 FROM tracks WHERE id = ?1)",
            [track_id],
            |r| r.get(0),
        )?;
        if exists {
            Ok(())
        } else {
            Err(Error::not_found("faixa", track_id))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Lyrics;
    use crate::testutil::*;

    // @test RF-12
    #[test]
    fn letra_grava_le_e_some_com_a_faixa() {
        let db = db();
        let t = add_track(&db, "Música", "Artista", 1000);
        let vazia = db.get_lyrics(&t.id).unwrap();
        assert_eq!((vazia.plain, vazia.synced), (None, None));

        let l = Lyrics {
            track_id: t.id.clone(),
            plain: Some("la la la".into()),
            synced: Some("[00:01.00]la".into()),
        };
        assert_eq!(db.set_lyrics(&l).unwrap(), l);
        // substituir
        let l2 = Lyrics {
            plain: Some("outra".into()),
            synced: None,
            ..l.clone()
        };
        assert_eq!(db.set_lyrics(&l2).unwrap().plain.as_deref(), Some("outra"));

        db.delete_track(&t.id).unwrap();
        let n: i64 = db
            .conn
            .query_row("SELECT count(*) FROM lyrics", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 0, "cascata remove a letra");
        assert_eq!(db.get_lyrics(&t.id).unwrap_err().code(), "not_found");
    }
}
