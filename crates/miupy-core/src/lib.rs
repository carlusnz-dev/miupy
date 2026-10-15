//! Núcleo de domínio e persistência do miupy (SQLite). Não depende de Tauri nem das outras
//! crates do workspace. Veja o `README.md` da crate e `docs/arquitetura/banco-de-dados.md`.

mod activity;
mod artists;
mod db;
mod downloads;
mod equalizer;
mod error;
mod lyrics;
mod model;
mod playlists;
mod profile;
mod recommendations;
mod search_history;
mod shuffle;
mod stats;
mod text;
mod time;
mod tracks;

#[cfg(test)]
mod testutil;

pub use db::Database;
pub use equalizer::{BUILTINS, EqPreset, EqPresetInput};
pub use error::{Error, Result};
pub use model::*;
pub use shuffle::{random_order, smart_order};
pub use stats::{PLAY_THRESHOLD_MS, TOP_LIMIT};
