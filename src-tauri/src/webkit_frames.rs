//! Taxa de quadros do WebKitGTK: por padrão ele prende a renderização perto de 60 fps
//! (`PreferPageRenderingUpdatesNear60FPS`), o que em telas de 120/144 Hz dá metade da
//! taxa ou um ritmo irregular. Com GPU, desliga a preferência para seguir o monitor de cada
//! tela. Em software fica ligada: sem o teto a rolagem oscila (p95 de 30–57 ms, sessão 08).
// @req RNF-13
use std::ffi::{CStr, c_char, c_int, c_void};
use tauri::Manager;
use webkit2gtk::WebViewExt;
use webkit2gtk::glib::translate::ToGlibPtr;

const NEAR_60FPS: &str = "PreferPageRenderingUpdatesNear60FPS";

unsafe extern "C" {
    fn webkit_settings_get_all_features() -> *mut c_void;
    fn webkit_feature_list_get_length(list: *mut c_void) -> usize;
    fn webkit_feature_list_get(list: *mut c_void, index: usize) -> *mut c_void;
    fn webkit_feature_list_unref(list: *mut c_void);
    fn webkit_feature_get_identifier(feature: *mut c_void) -> *const c_char;
    fn webkit_settings_set_feature_enabled(
        settings: *mut c_void,
        feature: *mut c_void,
        enabled: c_int,
    );
}

pub fn configure(app: &tauri::App, hardware: bool) {
    if !hardware {
        return;
    }
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let result = window.with_webview(|webview| {
        let Some(settings) = webview.inner().settings() else {
            log::warn!("quadros: WebKit sem configurações");
            return;
        };
        let raw: *mut webkit2gtk::ffi::WebKitSettings = settings.to_glib_none().0;
        // SAFETY: API pública do WebKitGTK 2.42+; a lista é liberada no fim e cada feature
        // vale enquanto a lista existir.
        let found = unsafe {
            let list = webkit_settings_get_all_features();
            let mut found = false;
            for i in 0..webkit_feature_list_get_length(list) {
                let feature = webkit_feature_list_get(list, i);
                let id = webkit_feature_get_identifier(feature);
                if !id.is_null() && CStr::from_ptr(id).to_bytes() == NEAR_60FPS.as_bytes() {
                    webkit_settings_set_feature_enabled(raw.cast(), feature, 0);
                    found = true;
                }
            }
            webkit_feature_list_unref(list);
            found
        };
        if !found {
            log::info!("quadros: {NEAR_60FPS} não existe neste WebKit");
        }
    });
    if let Err(e) = result {
        log::error!("quadros: não foi possível configurar o WebKit: {e}");
    }
}
