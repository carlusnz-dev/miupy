use miupy_core::{GpuMode, Profile, SettingsPatch};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::db::blocking;
use crate::dto::ImageUpload;
use crate::error::AppError;
use crate::state::AppState;

type R<T> = Result<T, AppError>;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsDto {
    lava_theme: miupy_core::LavaTheme,
    volume: f64,
    gpu_mode: GpuMode,
}
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SettingsPatchDto {
    lava_theme: Option<miupy_core::LavaTheme>,
    volume: Option<f64>,
    gpu_mode: Option<GpuMode>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuStatusDto {
    mode: GpuMode,
    env_override: Option<GpuMode>,
    hardware: bool,
    reason: String,
    restart_required: bool,
}

// @req RF-21
#[tauri::command]
pub async fn profile_set_avatar(
    state: State<'_, AppState>,
    image: Option<ImageUpload>,
) -> R<Profile> {
    let new_path = image
        .as_ref()
        .map(|i| crate::paths::save_upload(i, &state.dirs.root.join("avatars"), "avatar"))
        .transpose()?;
    let path = new_path.as_ref().map(|p| p.to_string_lossy().into_owned());
    let old = state
        .db
        .run(move |db| Ok(db.set_profile_avatar(path.as_deref())?))
        .await?;
    if let Some(old) = old {
        let p = std::path::PathBuf::from(old);
        if state.dirs.owns(&p) {
            let _ = std::fs::remove_file(p);
        }
    }
    state.db.run(|db| Ok(db.get_profile()?)).await
}

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
pub async fn settings_get(state: State<'_, AppState>) -> R<SettingsDto> {
    state
        .db
        .run(|db| {
            let s = db.get_settings()?;
            Ok(SettingsDto {
                lava_theme: s.lava_theme,
                volume: s.volume,
                gpu_mode: db.get_gpu_mode()?,
            })
        })
        .await
}

// @req RF-17 RF-02
#[tauri::command]
pub async fn settings_update(
    state: State<'_, AppState>,
    settings: SettingsPatchDto,
) -> R<SettingsDto> {
    let volume = settings.volume;
    let gpu_mode = settings.gpu_mode;
    let core = SettingsPatch {
        lava_theme: settings.lava_theme,
        volume,
    };
    let saved = state
        .db
        .run(move |db| {
            let s = db.update_settings(&core)?;
            if let Some(m) = gpu_mode {
                db.set_gpu_mode(m)?;
            }
            Ok(SettingsDto {
                lava_theme: s.lava_theme,
                volume: s.volume,
                gpu_mode: db.get_gpu_mode()?,
            })
        })
        .await?;
    // @req RNF-09: nova escolha descarta a marca de uma falha anterior, sem tocar na abertura atual.
    if gpu_mode.is_some()
        && let Err(error) = crate::gpu::discard_previous_failure(&state.dirs.root)
    {
        log::warn!("Falha ao descartar a sentinela antiga da GPU: {error}");
    }
    // Mudou o volume pelas configurações: o motor precisa ouvir também.
    if let Some(v) = volume {
        let player = state.player.clone();
        blocking(move || player.set_volume(v).map(|_| ())).await?;
    }
    Ok(saved)
}

// @req RNF-09
#[tauri::command]
pub async fn system_gpu_status(state: State<'_, AppState>) -> R<GpuStatusDto> {
    let mode = state.db.run(|db| Ok(db.get_gpu_mode()?)).await?;
    let override_mode = std::env::var("MIUPY_GPU")
        .ok()
        .and_then(|s| match s.as_str() {
            "on" => Some(GpuMode::On),
            "off" => Some(GpuMode::Off),
            "auto" => Some(GpuMode::Auto),
            _ => None,
        });
    let (hardware, reason) = crate::gpu::current_status();
    Ok(GpuStatusDto {
        mode,
        env_override: override_mode,
        hardware,
        reason,
        restart_required: override_mode.unwrap_or(mode) != crate::gpu::applied_mode(),
    })
}

// @req RNF-09
#[tauri::command]
pub async fn system_gpu_startup_confirmed() -> R<()> {
    crate::gpu::confirm_startup()?;
    Ok(())
}
