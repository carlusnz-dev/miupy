//! Acesso compartilhado ao banco. `Database` é `Send` mas não `Sync` e é síncrono: fica atrás
//! de um `Mutex` e, em código assíncrono, só é tocado dentro de `spawn_blocking`.
//! Nunca segure o lock através de um `.await`.

use std::sync::{Arc, Mutex, MutexGuard};

use miupy_core::Database;

use crate::error::{AppError, AppResult};

#[derive(Clone)]
pub struct Db(Arc<Mutex<Database>>);

impl Db {
    pub fn new(db: Database) -> Self {
        Db(Arc::new(Mutex::new(db)))
    }

    /// Lock do banco; um pânico em outra thread não deve travar o app inteiro.
    pub fn lock(&self) -> MutexGuard<'_, Database> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Executa uma operação curta no banco na thread atual.
    pub fn with<T>(&self, f: impl FnOnce(&Database) -> T) -> T {
        f(&self.lock())
    }

    /// Executa no pool de bloqueio (consultas que podem demorar) sem prender o runtime.
    pub async fn run<T, F>(&self, f: F) -> AppResult<T>
    where
        T: Send + 'static,
        F: FnOnce(&Database) -> AppResult<T> + Send + 'static,
    {
        let db = self.clone();
        tokio::task::spawn_blocking(move || f(&db.lock()))
            .await
            .map_err(AppError::from)?
    }
}

/// Roda trabalho síncrono no pool de bloqueio (áudio, disco) sem prender o runtime.
pub async fn blocking<T, F>(f: F) -> AppResult<T>
where
    T: Send + 'static,
    F: FnOnce() -> AppResult<T> + Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(AppError::from)?
}
