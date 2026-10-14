//! Conexão SQLite, PRAGMAs, migrações e relógio injetável.

use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rusqlite::Connection;
use rusqlite_migration::{M, Migrations};

use crate::error::Result;
use crate::text::register_collation;

/// Migrações versionadas (`PRAGMA user_version`). Nunca edite uma já publicada: acrescente
/// uma nova no fim da lista.
// @req RNF-01
fn migrations() -> Migrations<'static> {
    Migrations::new(vec![M::up(include_str!("../migrations/0001_init.sql"))])
}

/// Dono da conexão SQLite e ponto de entrada de toda a API de persistência. Os métodos por
/// domínio estão em `impl Database` nos módulos `tracks`, `playlists`, `downloads`,
/// `activity`, `stats`, `profile`, `artists` e `lyrics`.
///
/// A API é síncrona e `Database` é `Send` mas não `Sync`: quem chama guarda em um `Mutex`
/// e cuida das threads.
pub struct Database {
    pub(crate) conn: Connection,
    clock: Box<dyn Fn() -> i64 + Send>,
}

fn system_now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

impl Database {
    // @req RNF-01
    /// Abre (criando, se preciso, o arquivo e a pasta) e migra o banco em `path`.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        Self::init(Connection::open(path)?)
    }

    // @req RNF-01
    /// Banco em memória, descartado ao fechar. Usado nos testes.
    pub fn open_in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(mut conn: Connection) -> Result<Self> {
        conn.busy_timeout(Duration::from_secs(5))?;
        conn.pragma_update(None, "foreign_keys", true)?;
        // WAL: leitores não bloqueiam o escritor. Em memória o SQLite devolve "memory".
        conn.pragma_update_and_check(None, "journal_mode", "WAL", |_| Ok(()))?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        register_collation(&conn)?;
        migrations().to_latest(&mut conn)?;
        Ok(Database {
            conn,
            clock: Box::new(system_now_ms),
        })
    }

    /// Substitui o relógio (epoch ms). Para testes determinísticos.
    pub fn set_clock(&mut self, clock: impl Fn() -> i64 + Send + 'static) {
        self.clock = Box::new(clock);
    }

    /// Agora, em epoch ms UTC, segundo o relógio da instância.
    pub fn now_ms(&self) -> i64 {
        (self.clock)()
    }

    /// Versão atual do esquema (`user_version`).
    pub fn schema_version(&self) -> Result<i64> {
        Ok(self
            .conn
            .pragma_query_value(None, "user_version", |r| r.get(0))?)
    }

    /// Executa `f` dentro de um SAVEPOINT: tudo ou nada, e pode aninhar.
    pub(crate) fn in_tx<T>(&self, f: impl FnOnce() -> Result<T>) -> Result<T> {
        self.conn.execute_batch("SAVEPOINT miupy")?;
        match f() {
            Ok(v) => {
                self.conn.execute_batch("RELEASE miupy")?;
                Ok(v)
            }
            Err(e) => {
                let _ = self.conn.execute_batch("ROLLBACK TO miupy; RELEASE miupy");
                Err(e)
            }
        }
    }
}

/// Novo ID (UUID v7 em texto).
pub(crate) fn new_id() -> String {
    uuid::Uuid::now_v7().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // @test RNF-01
    #[test]
    fn migracoes_sao_validas() {
        migrations().validate().unwrap();
    }

    // @test RNF-01
    #[test]
    fn abre_em_memoria_com_pragmas_e_versao() {
        let db = Database::open_in_memory().unwrap();
        assert_eq!(db.schema_version().unwrap(), 1);
        let fk: i64 = db
            .conn
            .pragma_query_value(None, "foreign_keys", |r| r.get(0))
            .unwrap();
        assert_eq!(fk, 1);
    }

    // @test RNF-01
    #[test]
    fn fts5_esta_disponivel() {
        let db = Database::open_in_memory().unwrap();
        let n: i64 = db
            .conn
            .query_row("SELECT count(*) FROM tracks_fts", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 0);
    }

    // @test RNF-01
    #[test]
    fn abre_arquivo_em_wal_e_reabre_sem_remigrar() {
        let dir = std::env::temp_dir().join(format!("miupy-core-test-{}", new_id()));
        let path = dir.join("sub").join("miupy.db");
        {
            let db = Database::open(&path).unwrap();
            let mode: String = db
                .conn
                .pragma_query_value(None, "journal_mode", |r| r.get(0))
                .unwrap();
            assert_eq!(mode, "wal");
            db.update_profile(&crate::Profile {
                display_name: "Carlos".into(),
                username: "carlos".into(),
                email: String::new(),
                phone: String::new(),
                avatar_path: None,
            })
            .unwrap();
        }
        let db = Database::open(&path).unwrap();
        assert_eq!(db.schema_version().unwrap(), 1);
        assert_eq!(db.get_profile().unwrap().display_name, "Carlos");
        drop(db);
        let _ = std::fs::remove_dir_all(dir);
    }

    // @test RNF-01
    #[test]
    fn in_tx_desfaz_tudo_no_erro() {
        let db = Database::open_in_memory().unwrap();
        let r: Result<()> = db.in_tx(|| {
            db.conn.execute(
                "INSERT INTO artists (id, name, name_key, created_at) VALUES ('a','A','a',0)",
                [],
            )?;
            Err(crate::Error::invalid("falha proposital"))
        });
        assert!(r.is_err());
        let n: i64 = db
            .conn
            .query_row("SELECT count(*) FROM artists", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 0);
    }
}
