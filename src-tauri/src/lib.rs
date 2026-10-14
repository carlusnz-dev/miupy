//! miupy: casca Tauri. Implementa o contrato de `docs/arquitetura/ipc.md` sobre as crates
//! `miupy-core` (banco), `miupy-audio` (motor de reprodução) e `miupy-media` (YouTube,
//! metadados e tags). É a única parte do projeto que conhece o Tauri.

mod commands;
mod db;
mod downloads;
mod dto;
mod error;
mod events;
mod gpu;
mod listening;
mod metadata;
mod paths;
mod player;
mod queue;
mod recommendations;
mod state;
mod tz;
mod youtube;

use std::sync::Arc;

use miupy_media::{MetadataConfig, MetadataProvider, OnlineMetadata, YtDlpResolver};
use tauri::{Manager, RunEvent};

use commands::{
    download, library, metadata as meta, player as pl, playlist, profile, recommendations as rec,
    stats, youtube as yt,
};
use events::TauriEvents;
use state::{AppState, Parts, default_backend};

/// Contato do User-Agent exigido pelo MusicBrainz: `MIUPY_CONTACT`, senão o e-mail do perfil,
/// senão um texto genérico.
fn metadata_contact(state_db: &db::Db) -> String {
    if let Ok(c) = std::env::var("MIUPY_CONTACT")
        && !c.trim().is_empty()
    {
        return c;
    }
    state_db
        .with(|db| db.get_profile())
        .ok()
        .map(|p| p.email)
        .filter(|e| !e.trim().is_empty())
        .unwrap_or_else(|| "miupy (app de uso pessoal)".into())
}

fn build_state(app: &tauri::AppHandle) -> Result<AppState, Box<dyn std::error::Error>> {
    let dirs = paths::Dirs::new(app.path().app_data_dir()?);
    dirs.create_all()?;

    // O contato do User-Agent vem do perfil, então o banco é aberto uma vez antes só para ler.
    let contact = {
        let probe = db::Db::new(miupy_core::Database::open(&dirs.db_file)?);
        metadata_contact(&probe)
    };
    // UMA instância de OnlineMetadata: o limitador de 1 req/s do MusicBrainz é por instância.
    let provider: Arc<dyn MetadataProvider> =
        Arc::new(OnlineMetadata::new(MetadataConfig::new(contact))?);

    // Escopo do protocolo `asset`: só capas e áudio sob o diretório de dados do app.
    let scope = app.asset_protocol_scope();
    scope.allow_directory(&dirs.covers, true)?;
    scope.allow_directory(&dirs.audio, true)?;

    let state = AppState::new(Parts {
        dirs,
        events: Arc::new(TauriEvents(app.clone())),
        backend: default_backend(),
        resolver: Arc::new(YtDlpResolver::new()),
        provider,
    })?;
    Ok(state)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "linux")]
    let gpu_decision = gpu::configure();

    let app = tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .setup(move |app| {
            #[cfg(target_os = "linux")]
            log::info!("GPU: {}", gpu_decision);
            let state = build_state(app.handle())?;
            state.start();
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            pl::player_get_state,
            pl::player_get_queue,
            pl::player_play,
            pl::player_pause,
            pl::player_resume,
            pl::player_stop,
            pl::player_seek,
            pl::player_next,
            pl::player_previous,
            pl::player_set_volume,
            pl::player_set_muted,
            library::library_list_tracks,
            library::library_list_tracks_by_artist,
            library::library_get_track,
            library::library_set_favorite,
            library::library_delete_track,
            playlist::playlist_list,
            playlist::playlist_create,
            playlist::playlist_update,
            playlist::playlist_delete,
            playlist::playlist_get_tracks,
            playlist::playlist_add_tracks,
            playlist::playlist_remove_tracks,
            playlist::playlist_move_track,
            yt::youtube_search,
            yt::youtube_add_to_library,
            download::download_enqueue,
            download::download_enqueue_playlist,
            download::download_list,
            download::download_clear_finished,
            download::download_cancel,
            meta::metadata_refresh_track,
            meta::metadata_get_artist,
            meta::metadata_get_lyrics,
            stats::stats_library,
            stats::stats_wrapped,
            stats::stats_wrapped_years,
            stats::stats_taste,
            rec::recommendations_list,
            rec::recommendations_refresh,
            rec::recommendations_dismiss,
            profile::profile_get,
            profile::profile_update,
            profile::settings_get,
            profile::settings_update,
        ])
        .build(tauri::generate_context!())
        .expect("falha ao montar o app Tauri");

    app.run(|handle, event| {
        // @req RF-13: ao fechar o app, a reprodução em curso é registrada como `Stopped`.
        if let RunEvent::Exit = event
            && let Some(state) = handle.try_state::<AppState>()
        {
            state.player.shutdown();
        }
    });
}
