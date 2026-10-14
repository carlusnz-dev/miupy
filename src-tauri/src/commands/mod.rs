//! Comandos Tauri: a camada fina que implementa `docs/arquitetura/ipc.md`. Cada comando só
//! converte argumentos, chama a lógica testável dos outros módulos e emite eventos.
//!
//! Argumentos chegam em camelCase (o Tauri converte `track_id` em `trackId`); erros voltam como
//! `{ code, message }` (`AppError`).

pub mod download;
pub mod library;
pub mod metadata;
pub mod player;
pub mod playlist;
pub mod profile;
pub mod stats;
pub mod youtube;
