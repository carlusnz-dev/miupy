//! Ponte entre o contrato de áudio e o Media3 do Android.

use miupy_audio::volume_to_gain;

#[cfg(target_os = "android")]
use crossbeam_channel::{Receiver, Sender, unbounded};
#[cfg(target_os = "android")]
use miupy_audio::{
    AudioBackend, AudioError, AudioEvent, PlaybackState, PlaybackStatus, StreamOptions,
};
#[cfg(target_os = "android")]
use serde::Deserialize;
#[cfg(target_os = "android")]
use serde_json::json;
#[cfg(target_os = "android")]
use std::{
    path::Path,
    sync::{Arc, Mutex},
};
#[cfg(target_os = "android")]
use tauri::{
    Runtime,
    ipc::{Channel, InvokeResponseBody},
    plugin::PluginHandle,
};

// @req RF-02 RF-28 RF-44
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub(crate) fn effective_volume(volume: f32, muted: bool, gain_db: f32) -> f32 {
    if muted {
        0.0
    } else {
        volume_to_gain(volume) * 10f32.powf(gain_db.min(0.0) / 20.0)
    }
}

/// Interpola os ganhos do contrato na frequência central das bandas do dispositivo.
// @req RF-29 RF-44
#[allow(dead_code)] // Aplicado pelo audiofx.Equalizer na segunda metade.
pub(crate) fn map_eq_bands(centers_hz: &[u32], gains_db: [f32; 8]) -> Vec<f32> {
    const HZ: [u32; 8] = [60, 170, 310, 600, 1000, 3000, 6000, 12000];
    centers_hz
        .iter()
        .map(|&hz| {
            let i = HZ.partition_point(|&center| center < hz);
            if i == 0 {
                return gains_db[0];
            }
            if i == HZ.len() {
                return gains_db[7];
            }
            let low = HZ[i - 1] as f32;
            let high = HZ[i] as f32;
            let part = (hz as f32 - low) / (high - low);
            gains_db[i - 1] + (gains_db[i] - gains_db[i - 1]) * part
        })
        .collect()
}

#[cfg(target_os = "android")]
mod mobile {
    use super::*;
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct MobileEvent {
        kind: String,
        #[serde(default)]
        status: Option<String>,
        #[serde(default)]
        position_ms: u64,
        #[serde(default)]
        duration_ms: u64,
        #[serde(default)]
        message: Option<String>,
    }

    struct Inner {
        state: PlaybackState,
        gain_db: f32,
        #[allow(dead_code)] // reservado para o audiofx.Equalizer da segunda metade
        eq_gains_db: [f32; 8],
    }

    // @req RF-44 RF-18
    pub struct AndroidAudioBackend<R: Runtime> {
        plugin: PluginHandle<R>,
        inner: Arc<Mutex<Inner>>,
        sender: Sender<AudioEvent>,
        receiver: Receiver<AudioEvent>,
        _channel: Channel<serde_json::Value>,
    }

    impl<R: Runtime> AndroidAudioBackend<R> {
        pub fn new(plugin: PluginHandle<R>) -> miupy_audio::Result<Self> {
            let (sender, receiver) = unbounded();
            let inner = Arc::new(Mutex::new(Inner {
                state: PlaybackState::default(),
                gain_db: 0.0,
                eq_gains_db: [0.0; 8],
            }));
            let event_inner = inner.clone();
            let event_sender = sender.clone();
            let channel = Channel::new(move |body| {
                if let InvokeResponseBody::Json(raw) = body {
                    match serde_json::from_str::<MobileEvent>(&raw) {
                        Ok(event) => handle_mobile_event(&event_inner, &event_sender, event),
                        Err(error) => log::warn!("evento de áudio Android inválido: {error}"),
                    }
                }
                Ok(())
            });
            plugin
                .run_mobile_plugin::<()>("listen", json!({ "channel": channel.clone() }))
                .map_err(|e| AudioError::Unsupported(e.to_string()))?;
            Ok(Self {
                plugin,
                inner,
                sender,
                receiver,
                _channel: channel,
            })
        }

        fn call(&self, name: &str, args: serde_json::Value) -> miupy_audio::Result<()> {
            self.plugin
                .run_mobile_plugin::<()>(name, args)
                .map_err(|e| AudioError::Unsupported(e.to_string()))
        }

        fn update(&self, change: impl FnOnce(&mut Inner)) {
            let mut inner = self.inner.lock().unwrap();
            change(&mut inner);
            let state = inner.state.clone();
            drop(inner);
            let _ = self.sender.send(AudioEvent::StateChanged(state));
        }

        fn apply_volume(&self) -> miupy_audio::Result<()> {
            let inner = self.inner.lock().unwrap();
            let volume = effective_volume(inner.state.volume, inner.state.muted, inner.gain_db);
            drop(inner);
            self.call("volume", json!({ "volume": volume }))
        }
    }

    fn handle_mobile_event(inner: &Mutex<Inner>, sender: &Sender<AudioEvent>, event: MobileEvent) {
        match event.kind.as_str() {
            "state" => {
                let mut guard = inner.lock().unwrap();
                guard.state.status = match event.status.as_deref() {
                    Some("loading") => PlaybackStatus::Loading,
                    Some("playing") => PlaybackStatus::Playing,
                    Some("paused") => PlaybackStatus::Paused,
                    _ => PlaybackStatus::Stopped,
                };
                guard.state.position_ms = event.position_ms;
                if event.duration_ms > 0 {
                    guard.state.duration_ms = event.duration_ms;
                }
                let state = guard.state.clone();
                drop(guard);
                let _ = sender.send(AudioEvent::StateChanged(state));
            }
            "progress" => {
                let mut guard = inner.lock().unwrap();
                guard.state.position_ms = event.position_ms;
                if event.duration_ms > 0 {
                    guard.state.duration_ms = event.duration_ms;
                }
                let _ = sender.send(AudioEvent::Progress {
                    position_ms: guard.state.position_ms,
                    duration_ms: guard.state.duration_ms,
                });
            }
            "ended" => {
                let mut guard = inner.lock().unwrap();
                guard.state.status = PlaybackStatus::Stopped;
                guard.state.position_ms = event.position_ms;
                let state = guard.state.clone();
                drop(guard);
                let _ = sender.send(AudioEvent::StateChanged(state));
                let _ = sender.send(AudioEvent::TrackEnded);
            }
            "error" => {
                let _ = sender.send(AudioEvent::Error {
                    message: event
                        .message
                        .unwrap_or_else(|| "Falha ao reproduzir áudio".into()),
                });
            }
            _ => log::warn!("evento de áudio Android desconhecido: {}", event.kind),
        }
    }

    impl<R: Runtime> AudioBackend for AndroidAudioBackend<R> {
        fn load(&self, path: &Path) -> miupy_audio::Result<()> {
            if !path.is_file() {
                return Err(AudioError::Io {
                    path: path.to_owned(),
                    source: std::io::Error::from(std::io::ErrorKind::NotFound),
                });
            }
            self.call("load", json!({ "path": path.to_string_lossy() }))?;
            self.update(|g| {
                g.state.path = Some(path.to_owned());
                g.state.url = None;
                g.state.position_ms = 0;
                g.state.duration_ms = 0;
                g.state.status = PlaybackStatus::Paused;
            });
            Ok(())
        }
        fn load_url(&self, url: &str, options: StreamOptions) -> miupy_audio::Result<()> {
            let headers: std::collections::HashMap<_, _> = options.headers.into_iter().collect();
            self.call("loadUrl", json!({ "url": url, "headers": headers }))?;
            self.update(|g| {
                g.state.path = None;
                g.state.url = Some(url.into());
                g.state.position_ms = 0;
                g.state.duration_ms = options.duration_ms;
                g.state.status = PlaybackStatus::Loading;
            });
            Ok(())
        }
        fn play(&self) -> miupy_audio::Result<()> {
            if self.state().path.is_none() && self.state().url.is_none() {
                return Err(AudioError::NothingLoaded);
            }
            self.call("play", json!({}))
        }
        fn pause(&self) -> miupy_audio::Result<()> {
            self.call("pause", json!({}))
        }
        fn resume(&self) -> miupy_audio::Result<()> {
            self.play()
        }
        fn stop(&self) -> miupy_audio::Result<()> {
            self.call("stop", json!({}))?;
            self.update(|g| {
                g.state.status = PlaybackStatus::Stopped;
                g.state.path = None;
                g.state.url = None;
                g.state.position_ms = 0;
                g.state.duration_ms = 0;
            });
            Ok(())
        }
        fn seek(&self, position_ms: u64) -> miupy_audio::Result<()> {
            let duration = self.state().duration_ms;
            let position_ms = if duration == 0 {
                position_ms
            } else {
                position_ms.min(duration)
            };
            self.call("seek", json!({ "positionMs": position_ms }))?;
            self.update(|g| g.state.position_ms = position_ms);
            Ok(())
        }
        fn set_volume(&self, volume: f32) -> miupy_audio::Result<()> {
            if volume.is_finite() {
                self.update(|g| g.state.volume = volume.clamp(0.0, 1.0));
                self.apply_volume()?;
            }
            Ok(())
        }
        fn set_normalization_gain_db(&self, gain_db: f32) -> miupy_audio::Result<()> {
            if gain_db.is_finite() {
                self.inner.lock().unwrap().gain_db = gain_db;
                self.apply_volume()?;
            }
            Ok(())
        }
        fn set_equalizer(&self, gains_db: [f32; 8]) -> miupy_audio::Result<()> {
            self.inner.lock().unwrap().eq_gains_db = gains_db;
            Ok(())
        }
        fn set_muted(&self, muted: bool) -> miupy_audio::Result<()> {
            self.update(|g| g.state.muted = muted);
            self.apply_volume()
        }
        fn state(&self) -> PlaybackState {
            self.inner.lock().unwrap().state.clone()
        }
        fn events(&self) -> Receiver<AudioEvent> {
            self.receiver.clone()
        }
    }
}

#[cfg(target_os = "android")]
pub use mobile::AndroidAudioBackend;

#[cfg(test)]
mod tests {
    use super::*;
    // @test RF-02 RF-28 RF-44
    #[test]
    fn volume_so_atenua() {
        assert_eq!(effective_volume(0.5, false, 6.0), 0.25);
        assert!((effective_volume(0.5, false, -6.0) - 0.1253).abs() < 0.001);
        assert_eq!(effective_volume(0.8, true, -6.0), 0.0);
    }
    // @test RF-29 RF-44
    #[test]
    fn bandas_interpolam_e_limitam_extremos() {
        let values = map_eq_bands(
            &[30, 60, 115, 170, 16000],
            [0., 10., 0., 0., 0., 0., 0., -3.],
        );
        assert_eq!(values, vec![0., 0., 5., 10., -3.]);
    }
}
