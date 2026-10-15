//! Histórico de buscas no YouTube (RF-34).

use rusqlite::params;

use crate::db::Database;
use crate::error::Result;
use crate::text::name_key;

/// Quantas consultas o banco guarda.
pub const SEARCH_HISTORY_KEEP: usize = 50;
/// Quantas consultas a listagem devolve.
pub const SEARCH_HISTORY_SHOWN: usize = 10;

impl Database {
    // @req RF-34
    /// Grava a consulta (aparada; vazia é ignorada). Repetir, sem diferenciar caixa nem acento,
    /// sobe para o topo e guarda o texto da última digitação. Mantém só as 50 mais recentes.
    pub fn record_search(&self, query: &str) -> Result<()> {
        let query = query.split_whitespace().collect::<Vec<_>>().join(" ");
        if query.is_empty() {
            return Ok(());
        }
        let now = self.now_ms();
        self.in_tx(|| {
            self.conn.execute(
                "INSERT INTO search_history (query_key, query, searched_at) VALUES (?1, ?2, ?3)
                 ON CONFLICT(query_key) DO UPDATE SET query = ?2, searched_at = ?3",
                params![name_key(&query), query, now],
            )?;
            self.conn.execute(
                "DELETE FROM search_history WHERE query_key NOT IN
                 (SELECT query_key FROM search_history ORDER BY searched_at DESC, rowid DESC LIMIT ?1)",
                [SEARCH_HISTORY_KEEP as i64],
            )?;
            Ok(())
        })
    }

    // @req RF-34
    /// As 10 consultas mais recentes, da mais nova para a mais antiga.
    pub fn recent_searches(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT query FROM search_history ORDER BY searched_at DESC, rowid DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map([SEARCH_HISTORY_SHOWN as i64], |r| r.get(0))?;
        Ok(rows.collect::<std::result::Result<Vec<String>, _>>()?)
    }

    // @req RF-34
    /// Apaga todo o histórico de buscas.
    pub fn clear_search_history(&self) -> Result<()> {
        self.conn.execute("DELETE FROM search_history", [])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::testutil::db;

    // @test RF-34
    #[test]
    fn grava_apara_e_ignora_vazia() {
        let db = db();
        db.record_search("   ").unwrap();
        assert!(db.recent_searches().unwrap().is_empty());
        db.record_search("  daft   punk ").unwrap();
        assert_eq!(db.recent_searches().unwrap(), ["daft punk"]);
    }

    // @test RF-34
    #[test]
    fn repetir_sobe_ao_topo_sem_duplicar_e_guarda_a_ultima_digitacao() {
        let db = db();
        for q in ["Daft Punk", "Björk", "Radiohead"] {
            db.record_search(q).unwrap();
        }
        db.record_search("daft PUNK").unwrap();
        assert_eq!(
            db.recent_searches().unwrap(),
            ["daft PUNK", "Radiohead", "Björk"]
        );
        db.record_search("bjork").unwrap();
        assert_eq!(db.recent_searches().unwrap()[0], "bjork");
        assert_eq!(db.recent_searches().unwrap().len(), 3);
    }

    // @test RF-34
    #[test]
    fn guarda_50_e_lista_as_10_mais_recentes() {
        let db = db();
        for i in 0..51 {
            db.record_search(&format!("busca {i}")).unwrap();
        }
        let total: i64 = db
            .conn
            .query_row("SELECT COUNT(*) FROM search_history", [], |r| r.get(0))
            .unwrap();
        assert_eq!(total, 50);
        let kept_oldest: bool = db
            .conn
            .query_row(
                "SELECT EXISTS (SELECT 1 FROM search_history WHERE query = 'busca 0')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(!kept_oldest);
        let recent = db.recent_searches().unwrap();
        assert_eq!(recent.len(), 10);
        assert_eq!(recent[0], "busca 50");
        assert_eq!(recent[9], "busca 41");
    }

    // @test RF-34
    #[test]
    fn limpar_apaga_tudo() {
        let db = db();
        db.record_search("a").unwrap();
        db.record_search("b").unwrap();
        db.clear_search_history().unwrap();
        assert!(db.recent_searches().unwrap().is_empty());
        db.record_search("c").unwrap();
        assert_eq!(db.recent_searches().unwrap(), ["c"]);
    }
}
