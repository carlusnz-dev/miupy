use miupy_core::{Profile, Settings, SettingsPatch};
use tauri::State;

use crate::db::blocking;
use crate::error::AppError;
use crate::state::AppState;

type R<T> = Result<T, AppError>;

// @req RF-16
#[tauri::command]
pub async fn profile_get(state: State<'_, AppState>) -> R<Profile> {
    state.db.run(|db| Ok(db.get_profile()?)).await
}

// @req RF-16
#[tauri::command]
pub async fn profile_update(state: State<'_, AppState>, profile: Profile) -> R<Profile> {
    state
        .db
        .run(move |db| {
            let saved = db.update_profile(&profile)?;
            db.log_action("profile_updated", Some("profile"), None, None)?;
            Ok(saved)
        })
        .await
}

// @req RF-17
#[tauri::command]
pub async fn settings_get(state: State<'_, AppState>) -> R<Settings> {
    state.db.run(|db| Ok(db.get_settings()?)).await
}

// @req RF-17 RF-02
#[tauri::command]
pub async fn settings_update(state: State<'_, AppState>, settings: SettingsPatch) -> R<Settings> {
    let volume = settings.volume;
    let saved = state
        .db
        .run(move |db| Ok(db.update_settings(&settings)?))
        .await?;
    // Mudou o volume pelas configurações: o motor precisa ouvir também.
    if let Some(v) = volume {
        let player = state.player.clone();
        blocking(move || player.set_volume(v).map(|_| ())).await?;
    }
    Ok(saved)
}
