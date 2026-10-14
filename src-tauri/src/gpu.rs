//! Seleção do renderizador WebKitGTK antes de iniciar qualquer thread.
use std::sync::OnceLock;
static APPLIED: OnceLock<(bool, String, &'static str)> = OnceLock::new();

/// Decisão pura, separada da leitura do ambiente para permitir testar combinações.
// @req RNF-09
pub fn decision(
    mode: Option<&str>,
    webkit_override: Option<&str>,
    wayland: bool,
    nvidia: bool,
) -> (&'static str, &'static str) {
    if webkit_override.is_some() {
        return (
            "preservado",
            "WEBKIT_DISABLE_DMABUF_RENDERER definido pelo usuário",
        );
    }
    match mode.unwrap_or("auto").to_ascii_lowercase().as_str() {
        "on" => ("gpu", "MIUPY_GPU=on"),
        "off" => ("software", "MIUPY_GPU=off"),
        _ if wayland && nvidia => ("software", "Wayland com driver NVIDIA detectado"),
        _ => ("gpu", "detecção automática sem combinação problemática"),
    }
}

fn effective_mode<'a>(environment: Option<&'a str>, saved: Option<&'a str>) -> Option<&'a str> {
    environment.or(saved)
}

#[cfg(target_os = "linux")]
pub fn configure_mode(saved_mode: Option<&str>) -> String {
    let env_mode = std::env::var("MIUPY_GPU").ok();
    let override_value = std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER");
    let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some();
    let nvidia = std::path::Path::new("/proc/driver/nvidia").exists()
        || std::path::Path::new("/sys/module/nvidia").exists();
    let selected = effective_mode(env_mode.as_deref(), saved_mode);
    let (chosen, reason) = decision(
        selected,
        override_value.as_ref().map(|_| "definido"),
        wayland,
        nvidia,
    );
    if override_value.is_none() {
        // SAFETY: executado antes da criação do runtime Tauri e de qualquer thread do app.
        unsafe {
            std::env::set_var(
                "WEBKIT_DISABLE_DMABUF_RENDERER",
                if chosen == "software" { "1" } else { "0" },
            )
        };
    }
    let hardware = match override_value.as_deref().and_then(|v| v.to_str()) {
        Some("0") => true,
        Some("1") => false,
        _ => chosen == "gpu",
    };
    let _ = APPLIED.set((
        hardware,
        reason.to_owned(),
        match selected.unwrap_or("auto") {
            "on" => "on",
            "off" => "off",
            _ => "auto",
        },
    ));
    format!("{chosen} ({reason})")
}

pub fn current_status() -> (bool, String) {
    APPLIED
        .get()
        .map(|(h, r, _)| (*h, r.clone()))
        .unwrap_or((false, "renderizador ainda não inicializado".into()))
}
pub fn applied_mode() -> miupy_core::GpuMode {
    match APPLIED.get().map(|x| x.2).unwrap_or("auto") {
        "on" => miupy_core::GpuMode::On,
        "off" => miupy_core::GpuMode::Off,
        _ => miupy_core::GpuMode::Auto,
    }
}

#[cfg(target_os = "linux")]
pub fn load_saved_mode() -> Option<String> {
    let root = std::env::var_os("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".local/share"))
        })?;
    miupy_core::Database::open(root.join("dev.miupy.app/miupy.db"))
        .ok()?
        .get_gpu_mode()
        .ok()
        .map(|m| m.as_str().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    // @test RNF-09
    #[test]
    fn decide_gpu_por_modo_e_ambiente() {
        assert_eq!(decision(None, None, false, false).0, "gpu");
        assert_eq!(decision(None, None, true, true).0, "software");
        assert_eq!(decision(Some("on"), None, true, true).0, "gpu");
        assert_eq!(decision(Some("off"), None, false, false).0, "software");
        assert_eq!(decision(Some("off"), Some("0"), true, true).0, "preservado");
        assert_eq!(effective_mode(Some("on"), Some("off")), Some("on"));
        assert_eq!(effective_mode(None, Some("off")), Some("off"));
    }
}
