// @req RF-05
use tauri::Manager;
use webkit2gtk::{SecurityManagerExt, WebContextExt, WebViewExt};

const ASSET_SCHEME: &str = "asset";

fn allow_asset_cors(register: impl FnOnce(&str)) {
    register(ASSET_SCHEME);
}

pub fn configure(app: &tauri::App) -> tauri::Result<()> {
    let window = app.get_webview_window("main").expect("janela principal");
    window.with_webview(|webview| {
        let context = webview.inner().context().expect("contexto WebKit");
        let manager = context
            .security_manager()
            .expect("gestor de segurança WebKit");
        // O wry marca o protocolo como seguro, mas WebKit bloqueia imagens de uma página
        // http(s) enquanto o esquema não for habilitado para requisições entre origens.
        // O handler do Tauri ainda verifica assetProtocol.scope e devolve CORS para a janela.
        allow_asset_cors(|scheme| manager.register_uri_scheme_as_cors_enabled(scheme));
        debug_assert!(manager.uri_scheme_is_cors_enabled(ASSET_SCHEME));
    })
}

// @test RF-05
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn asset_scheme_is_registered_for_cors() {
        let mut registered = None;
        allow_asset_cors(|scheme| registered = Some(scheme.to_owned()));
        assert_eq!(registered.as_deref(), Some(ASSET_SCHEME));
    }

    #[test]
    fn asset_protocol_remains_limited_and_allowed_by_both_csp_modes() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).expect("configuração Tauri");
        let security = &config["app"]["security"];
        assert_eq!(security["assetProtocol"]["enable"], true);
        let scope = security["assetProtocol"]["scope"]
            .as_array()
            .expect("escopo de arquivos");
        assert_eq!(
            scope,
            &[
                "$APPDATA/audio/**",
                "$APPDATA/covers/**",
                "$APPDATA/avatars/**",
                "$APPDATA/playlists/**"
            ]
            .map(serde_json::Value::from)
        );
        for mode in ["csp", "devCsp"] {
            let images = security[mode]["img-src"].as_str().expect("CSP de imagens");
            assert!(images.split_whitespace().any(|source| source == "asset:"));
        }
    }
}
