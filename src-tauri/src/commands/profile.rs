use miupy_core::{GpuMode, Normalization, Profile, SettingsPatch};
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
    normalization: Normalization,
    eq_preset_id: String,
    // @req RF-33
    sidebar_width: i64,
    // @req RF-30
    favorites_playlist_id: Option<String>,
}
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SettingsPatchDto {
    lava_theme: Option<miupy_core::LavaTheme>,
    volume: Option<f64>,
    gpu_mode: Option<GpuMode>,
    normalization: Option<Normalization>,
    eq_preset_id: Option<String>,
    // @req RF-33
    sidebar_width: Option<i64>,
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
                normalization: s.normalization,
                eq_preset_id: s.eq_preset_id,
                sidebar_width: s.sidebar_width,
                favorites_playlist_id: s.favorites_playlist_id,
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
        normalization: settings.normalization,
        eq_preset_id: settings.eq_preset_id.clone(),
        sidebar_width: settings.sidebar_width.map(|width| width.clamp(220, 400)),
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
                normalization: s.normalization,
                eq_preset_id: s.eq_preset_id,
                sidebar_width: s.sidebar_width,
                favorites_playlist_id: s.favorites_playlist_id,
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
    if settings.eq_preset_id.is_some() {
        let player = state.player.clone();
        blocking(move || player.refresh_equalizer()).await?;
    }
    if settings.normalization.is_some() {
        let player = state.player.clone();
        blocking(move || player.refresh_normalization()).await?;
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

// @req RF-29
#[tauri::command]
pub async fn equalizer_list_presets(state: State<'_, AppState>) -> R<Vec<miupy_core::EqPreset>> {
    state.db.run(|db| Ok(db.list_eq_presets()?)).await
}
// @req RF-29
#[tauri::command]
pub async fn equalizer_save_preset(
    state: State<'_, AppState>,
    preset: miupy_core::EqPresetInput,
) -> R<miupy_core::EqPreset> {
    let saved = state
        .db
        .run(move |db| Ok(db.save_eq_preset(&preset)?))
        .await?;
    let active = state
        .db
        .run(|db| Ok(db.get_settings()?.eq_preset_id))
        .await?;
    if active == saved.id {
        let player = state.player.clone();
        blocking(move || player.refresh_equalizer()).await?;
    }
    Ok(saved)
}
// @req RF-29
#[tauri::command]
pub async fn equalizer_delete_preset(state: State<'_, AppState>, preset_id: String) -> R<()> {
    let active = state
        .db
        .run(|db| Ok(db.get_settings()?.eq_preset_id))
        .await?;
    let changed = active == preset_id;
    state
        .db
        .run(move |db| Ok(db.delete_eq_preset(&preset_id)?))
        .await?;
    if changed {
        let player = state.player.clone();
        blocking(move || player.refresh_equalizer()).await?;
    }
    Ok(())
}

#[cfg(test)]
mod favorites_tests {
    use super::*;

    // @test RF-30
    #[test]
    fn settings_dto_expoe_playlist_de_favoritas() {
        let dto = SettingsDto {
            lava_theme: miupy_core::LavaTheme::Orange,
            volume: 0.8,
            gpu_mode: GpuMode::Auto,
            normalization: Normalization::Normal,
            eq_preset_id: "flat".into(),
            sidebar_width: 280,
            favorites_playlist_id: Some("p".into()),
        };
        assert_eq!(
            serde_json::to_value(dto).unwrap()["favoritesPlaylistId"],
            "p"
        );
    }
}
