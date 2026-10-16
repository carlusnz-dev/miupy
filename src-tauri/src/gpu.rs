//! Seleção do renderizador WebKitGTK antes de iniciar qualquer thread.
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const MARKER: &str = "gpu-startup-pending";
const CRASH_REASON: &str = "GPU desativada: a última abertura com aceleração caiu";

struct Applied {
    hardware: bool,
    reason: String,
    mode: &'static str,
    marker: Option<PathBuf>,
}
static APPLIED: OnceLock<Applied> = OnceLock::new();

#[derive(Debug, PartialEq)]
pub struct Decision {
    renderer: &'static str,
    reason: String,
    mode: &'static str,
    reset_saved: bool,
    mark_startup: bool,
}

/// Decide sem acessar ambiente, banco ou janela. A origem do modo faz parte da decisão.
// @req RNF-09
pub fn decision(
    saved: Option<&str>,
    environment: Option<&str>,
    webkit_override: Option<&str>,
    wayland: bool,
    nvidia: bool,
    webkit_version: Option<(u32, u32)>,
    previous_failed: bool,
) -> Decision {
    let selected = environment.or(saved).unwrap_or("auto").to_ascii_lowercase();
    let mode = match selected.as_str() {
        "on" => "on",
        "off" => "off",
        _ => "auto",
    };
    let risk = wayland && nvidia;
    let compatible_webkit = webkit_version.is_some_and(|(major, minor)| (major, minor) < (2, 54));
    if webkit_override.is_some() {
        return Decision {
            renderer: "preservado",
            reason: "WEBKIT_DISABLE_DMABUF_RENDERER definido pelo usuário".into(),
            mode,
            reset_saved: false,
            mark_startup: false,
        };
    }
    if environment.is_none()
        && risk
        && previous_failed
        && (saved == Some("on") || (mode == "auto" && compatible_webkit))
    {
        return Decision {
            renderer: "software",
            reason: CRASH_REASON.into(),
            mode: "auto",
            reset_saved: true,
            mark_startup: false,
        };
    }
    match mode {
        "on" => Decision {
            renderer: "gpu",
            reason: if environment.is_some() {
                "MIUPY_GPU=on"
            } else {
                "GPU ativada nas configurações"
            }
            .into(),
            mode,
            reset_saved: false,
            mark_startup: environment.is_none() && risk,
        },
        "off" => Decision {
            renderer: "software",
            reason: if environment.is_some() {
                "MIUPY_GPU=off"
            } else {
                "GPU desativada nas configurações"
            }
            .into(),
            mode,
            reset_saved: false,
            mark_startup: false,
        },
        _ if risk => match webkit_version {
            Some((major, minor)) if compatible_webkit => Decision {
                renderer: "gpu",
                reason: format!("WebKitGTK {major}.{minor} sem o problema de fluidez"),
                mode,
                reset_saved: false,
                mark_startup: environment.is_none(),
            },
            Some((major, minor)) => Decision {
                renderer: "software",
                reason: format!("WebKitGTK {major}.{minor} com Wayland e NVIDIA"),
                mode,
                reset_saved: false,
                mark_startup: false,
            },
            None => Decision {
                renderer: "software",
                reason: "versão do WebKitGTK desconhecida com Wayland e NVIDIA".into(),
                mode,
                reset_saved: false,
                mark_startup: false,
            },
        },
        _ => Decision {
            renderer: "gpu",
            reason: "detecção automática sem combinação problemática".into(),
            mode,
            reset_saved: false,
            mark_startup: false,
        },
    }
}

/// Persiste o recuo e arma a sentinela antes da janela. Falhas impedem a tentativa arriscada.
// @req RNF-09
fn prepare_startup(
    root: &Path,
    decision: &Decision,
) -> Result<Option<PathBuf>, Box<dyn std::error::Error>> {
    let marker = root.join(MARKER);
    if decision.reset_saved {
        miupy_core::Database::open(root.join("miupy.db"))?
            .set_gpu_mode(miupy_core::GpuMode::Auto)?;
        std::fs::remove_file(&marker)?;
    }
    if decision.mark_startup {
        std::fs::create_dir_all(root)?;
        std::fs::write(&marker, b"gpu-on\n")?;
        return Ok(Some(marker));
    }
    Ok(None)
}

#[cfg(target_os = "linux")]
// @req RNF-09
pub fn data_root() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))?;
    Some(base.join("dev.miupy.app"))
}

#[cfg(target_os = "linux")]
// @req RNF-09
pub fn configure_mode(saved_mode: Option<&str>, root: Option<&Path>) -> String {
    let env_mode = std::env::var("MIUPY_GPU").ok();
    let override_value = std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER");
    let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some();
    let nvidia =
        Path::new("/proc/driver/nvidia").exists() || Path::new("/sys/module/nvidia").exists();
    // @req RNF-09: consulta a biblioteca carregada antes de criar a janela.
    // SAFETY: as funções de versão do WebKitGTK não exigem contexto nem ponteiros de entrada.
    let webkit_version = unsafe {
        Some((
            webkit2gtk::ffi::webkit_get_major_version(),
            webkit2gtk::ffi::webkit_get_minor_version(),
        ))
    };
    let previous_failed = root.is_some_and(|r| r.join(MARKER).exists());
    let mut selected = decision(
        saved_mode,
        env_mode.as_deref(),
        override_value.as_ref().map(|_| "definido"),
        wayland,
        nvidia,
        webkit_version,
        previous_failed,
    );
    let marker = match root {
        Some(root) => match prepare_startup(root, &selected) {
            Ok(marker) => marker,
            Err(error) => {
                eprintln!("miupy: falha ao preparar a GPU: {error}");
                selected = Decision {
                    renderer: "software",
                    reason: "GPU desativada: falha ao preparar a abertura segura".into(),
                    mode: "auto",
                    reset_saved: false,
                    mark_startup: false,
                };
                None
            }
        },
        None if selected.mark_startup => {
            selected = Decision {
                renderer: "software",
                reason: "GPU desativada: diretório de dados indisponível".into(),
                mode: "auto",
                reset_saved: false,
                mark_startup: false,
            };
            None
        }
        None => None,
    };
    if override_value.is_none() {
        // SAFETY: executado antes da criação do runtime Tauri e de qualquer thread do app.
        unsafe {
            std::env::set_var(
                "WEBKIT_DISABLE_DMABUF_RENDERER",
                if selected.renderer == "software" {
                    "1"
                } else {
                    "0"
                },
            )
        };
    }
    let hardware = match override_value.as_deref().and_then(|v| v.to_str()) {
        Some("0") => true,
        Some("1") => false,
        _ => selected.renderer == "gpu",
    };
    let _ = APPLIED.set(Applied {
        hardware,
        reason: selected.reason.clone(),
        mode: selected.mode,
        marker,
    });
    format!("{} ({})", selected.renderer, selected.reason)
}

/// Chamado após a interface montar e permanecer aberta por alguns segundos.
// @req RNF-09
pub fn confirm_startup() -> std::io::Result<()> {
    if let Some(marker) = APPLIED.get().and_then(|a| a.marker.as_ref()) {
        match std::fs::remove_file(marker) {
            Ok(()) => (),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

/// Uma escolha explícita na interface descarta uma sentinela de uma tentativa anterior.
// @req RNF-09
pub fn discard_previous_failure(root: &Path) -> std::io::Result<()> {
    if APPLIED.get().is_some_and(|a| a.marker.is_some()) {
        return Ok(());
    }
    match std::fs::remove_file(root.join(MARKER)) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

pub fn current_status() -> (bool, String) {
    APPLIED
        .get()
        .map(|a| (a.hardware, a.reason.clone()))
        .unwrap_or((false, "renderizador ainda não inicializado".into()))
}
pub fn applied_mode() -> miupy_core::GpuMode {
    match APPLIED.get().map(|a| a.mode).unwrap_or("auto") {
        "on" => miupy_core::GpuMode::On,
        "off" => miupy_core::GpuMode::Off,
        _ => miupy_core::GpuMode::Auto,
    }
}

#[cfg(target_os = "linux")]
// @req RNF-09
pub fn load_saved_mode(root: Option<&Path>) -> Option<String> {
    miupy_core::Database::open(root?.join("miupy.db"))
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
        assert_eq!(
            decision(None, None, None, false, false, None, false).renderer,
            "gpu"
        );
        assert_eq!(
            decision(None, None, None, true, true, None, false).renderer,
            "software"
        );
        assert_eq!(
            decision(Some("on"), None, None, true, true, None, false).renderer,
            "gpu"
        );
        assert_eq!(
            decision(Some("off"), None, None, false, false, None, false).renderer,
            "software"
        );
        assert_eq!(
            decision(Some("off"), None, Some("0"), true, true, None, false).renderer,
            "preservado"
        );
        assert_eq!(
            decision(Some("off"), Some("on"), None, true, true, None, false).renderer,
            "gpu"
        );
    }

    // @test RNF-09
    #[test]
    fn automatico_decide_pela_versao_so_com_wayland_e_nvidia() {
        for (version, renderer, reason) in [
            (
                Some((2, 50)),
                "gpu",
                "WebKitGTK 2.50 sem o problema de fluidez",
            ),
            (
                Some((2, 53)),
                "gpu",
                "WebKitGTK 2.53 sem o problema de fluidez",
            ),
            (
                Some((2, 54)),
                "software",
                "WebKitGTK 2.54 com Wayland e NVIDIA",
            ),
            (
                Some((2, 55)),
                "software",
                "WebKitGTK 2.55 com Wayland e NVIDIA",
            ),
            (
                None,
                "software",
                "versão do WebKitGTK desconhecida com Wayland e NVIDIA",
            ),
        ] {
            let selected = decision(None, None, None, true, true, version, false);
            assert_eq!(selected.renderer, renderer);
            assert_eq!(selected.reason, reason);
            assert_eq!(selected.mark_startup, renderer == "gpu");
        }
        for (wayland, nvidia) in [(false, true), (true, false), (false, false)] {
            let selected = decision(None, None, None, wayland, nvidia, Some((2, 54)), false);
            assert_eq!(selected.renderer, "gpu");
            assert!(!selected.mark_startup);
        }
        let explicit_auto = decision(None, Some("auto"), None, true, true, Some((2, 50)), false);
        assert_eq!(explicit_auto.renderer, "gpu");
        assert!(!explicit_auto.mark_startup);
        assert_eq!(
            decision(Some("on"), None, None, true, true, Some((2, 54)), false).renderer,
            "gpu"
        );
        assert_eq!(
            decision(Some("off"), None, None, true, true, Some((2, 50)), false).renderer,
            "software"
        );
        assert_eq!(
            decision(None, None, Some("1"), true, true, Some((2, 50)), false).renderer,
            "preservado"
        );
    }

    // @test RNF-09
    #[test]
    fn sentinela_protege_gpu_automatico_apos_queda() {
        let dir = tempfile::tempdir().unwrap();
        let selected = decision(Some("auto"), None, None, true, true, Some((2, 50)), false);
        assert_eq!(selected.renderer, "gpu");
        assert_eq!(
            prepare_startup(dir.path(), &selected).unwrap(),
            Some(dir.path().join(MARKER))
        );
        assert!(dir.path().join(MARKER).exists());

        let recovered = decision(Some("auto"), None, None, true, true, Some((2, 50)), true);
        assert_eq!(recovered.renderer, "software");
        assert_eq!(recovered.reason, CRASH_REASON);
        assert!(recovered.reset_saved);
        assert!(!recovered.mark_startup);
        prepare_startup(dir.path(), &recovered).unwrap();
        assert!(!dir.path().join(MARKER).exists());
    }

    // @test RNF-09
    #[test]
    fn motivo_identifica_modo_salvo() {
        assert_eq!(
            decision(Some("on"), None, None, true, true, None, false).reason,
            "GPU ativada nas configurações"
        );
        assert_eq!(
            decision(Some("on"), Some("on"), None, true, true, None, false).reason,
            "MIUPY_GPU=on"
        );
    }

    // @test RNF-09
    #[test]
    fn recupera_falha_anterior_e_restaura_auto() {
        let dir = tempfile::tempdir().unwrap();
        let db = miupy_core::Database::open(dir.path().join("miupy.db")).unwrap();
        db.set_gpu_mode(miupy_core::GpuMode::On).unwrap();
        std::fs::write(dir.path().join(MARKER), b"gpu-on\n").unwrap();
        let selected = decision(Some("on"), None, None, true, true, None, true);
        assert_eq!(selected.renderer, "software");
        assert_eq!(selected.reason, CRASH_REASON);
        prepare_startup(dir.path(), &selected).unwrap();
        assert_eq!(db.get_gpu_mode().unwrap(), miupy_core::GpuMode::Auto);
        assert!(!dir.path().join(MARKER).exists());
    }

    // @test RNF-09
    #[test]
    fn variavel_explicitamente_liga_gpu_mesmo_apos_falha() {
        let selected = decision(Some("on"), Some("on"), None, true, true, None, true);
        assert_eq!(selected.renderer, "gpu");
        assert!(!selected.reset_saved);
        assert!(!selected.mark_startup);
    }

    // @test RNF-09
    #[test]
    fn sentinela_so_em_tentativa_arriscada_do_modo_salvo() {
        let dir = tempfile::tempdir().unwrap();
        for (wayland, nvidia, env) in [
            (false, true, None),
            (true, false, None),
            (true, true, Some("on")),
        ] {
            let selected = decision(Some("on"), env, None, wayland, nvidia, None, false);
            assert!(!selected.mark_startup);
            assert!(prepare_startup(dir.path(), &selected).unwrap().is_none());
            assert!(!dir.path().join(MARKER).exists());
        }
        let selected = decision(Some("on"), None, None, true, true, None, false);
        assert_eq!(
            prepare_startup(dir.path(), &selected).unwrap(),
            Some(dir.path().join(MARKER))
        );
        assert!(dir.path().join(MARKER).exists());
    }

    // @test RNF-09
    #[test]
    fn escolha_explicitamente_nova_descarta_falha_antiga() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(MARKER), b"gpu-on\n").unwrap();
        discard_previous_failure(dir.path()).unwrap();
        assert!(!dir.path().join(MARKER).exists());
    }
}
