//! Motor de reprodução do miupy.
//!
//! A trait [`AudioBackend`] toca um arquivo por vez. Implementações:
//! - `RodioBackend` (desktop, `cfg(not(target_os = "android"))`);
//! - [`NullBackend`] (testes e Android até existir o plugin Media3).
//!
//! No Android a reprodução real será um plugin Tauri em Kotlin com Media3; o
//! ponto de extensão é implementar `AudioBackend` por cima da ponte do plugin
//! e devolvê-lo em [`create_default_backend`].

mod error;
mod null;
#[cfg(not(target_os = "android"))]
mod rodio_backend;
mod types;

pub use error::{AudioError, Result};
pub use null::NullBackend;
#[cfg(not(target_os = "android"))]
pub use rodio_backend::RodioBackend;
pub use types::{
    AudioBackend, AudioEvent, PROGRESS_INTERVAL_MS, PlaybackState, PlaybackStatus, StreamOptions,
    volume_to_gain,
};

/// Backend padrão da plataforma: rodio no desktop; no Android, por enquanto, o
/// [`NullBackend`] (silencioso) até o plugin Media3 existir.
pub fn create_default_backend() -> Result<Box<dyn AudioBackend>> {
    #[cfg(not(target_os = "android"))]
    {
        Ok(Box::new(RodioBackend::new()?))
    }
    #[cfg(target_os = "android")]
    {
        // Ponto de extensão: trocar pelo backend que fala com o plugin Kotlin/Media3.
        Ok(Box::new(NullBackend::new()))
    }
}
