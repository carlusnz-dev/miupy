//! Seleção do renderizador WebKitGTK antes de iniciar qualquer thread.

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

#[cfg(target_os = "linux")]
pub fn configure() -> String {
    let mode = std::env::var("MIUPY_GPU").ok();
    let override_value = std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER");
    let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some();
    let nvidia = std::path::Path::new("/proc/driver/nvidia").exists()
        || std::path::Path::new("/sys/module/nvidia").exists();
    let (chosen, reason) = decision(
        mode.as_deref(),
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
    format!("{chosen} ({reason})")
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
    }
}
