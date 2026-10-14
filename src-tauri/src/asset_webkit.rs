// @req RF-05
use tauri::Manager;
use webkit2gtk::{SecurityManagerExt, WebContextExt, WebViewExt};

const ASSET_SCHEME: &str = "asset";

/// Habilita CORS do esquema `asset` no WebKitGTK. Sem isso, a página de dev
/// (`http://localhost:1420`) não carrega nenhuma imagem `asset://`. Uma falha aqui só
/// esconde as capas, então vai para o log em vez de derrubar o app.
pub fn configure(app: &tauri::App) {
    let Some(window) = app.get_webview_window("main") else {
        log::error!("asset: janela principal não encontrada; capas locais podem não carregar");
        return;
    };
    let result = window.with_webview(|webview| {
        let Some(manager) = webview
            .inner()
            .context()
            .and_then(|context| context.security_manager())
        else {
            log::error!("asset: WebKit sem gestor de segurança; capas locais podem não carregar");
            return;
        };
        // O wry marca o protocolo como seguro, mas WebKit bloqueia imagens de uma página
        // http(s) enquanto o esquema não for habilitado para requisições entre origens.
        // O handler do Tauri ainda verifica assetProtocol.scope e devolve CORS para a janela.
        manager.register_uri_scheme_as_cors_enabled(ASSET_SCHEME);
    });
    if let Err(e) = result {
        log::error!("asset: não foi possível configurar o WebKit: {e}");
    }
}

// @test RF-05
#[cfg(test)]
mod tests {
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
