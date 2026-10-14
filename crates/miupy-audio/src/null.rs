//! Backend nulo: simula o tempo sem tocar nada. Serve para testes da camada
//! de cima e para compilar onde não há motor real (Android, por enquanto).

use crate::error::{AudioError, Result};
use crate::types::{
    AudioBackend, AudioEvent, PROGRESS_INTERVAL_MS, PlaybackState, PlaybackStatus, StreamOptions,
};
use crossbeam_channel::{Receiver, Sender, unbounded};
use std::path::Path;
use std::sync::Mutex;

struct Inner {
    state: PlaybackState,
    next_duration_ms: u64,
    fail_loads: bool,
}

/// Backend de teste. O relógio é manual: nada avança até [`NullBackend::advance`].
pub struct NullBackend {
    inner: Mutex<Inner>,
    tx: Sender<AudioEvent>,
    rx: Receiver<AudioEvent>,
}

impl Default for NullBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl NullBackend {
    /// Faixas "carregadas" duram 3 minutos, salvo `set_next_duration_ms`.
    pub fn new() -> Self {
        let (tx, rx) = unbounded();
        Self {
            inner: Mutex::new(Inner {
                state: PlaybackState::default(),
                next_duration_ms: 180_000,
                fail_loads: false,
            }),
            tx,
            rx,
        }
    }

    /// Duração que o próximo `load` vai informar.
    pub fn set_next_duration_ms(&self, ms: u64) {
        self.lock().next_duration_ms = ms;
    }

    /// Faz os próximos `load` falharem (para testar tratamento de erro).
    pub fn set_fail_loads(&self, fail: bool) {
        self.lock().fail_loads = fail;
    }

    /// Avança o relógio simulado. Só tem efeito em `Playing`. Emite `Progress`
    /// e, se a faixa acabar, `StateChanged(Stopped)` + `TrackEnded`.
    // @req RF-01
    pub fn advance(&self, ms: u64) {
        let mut g = self.lock();
        if g.state.status != PlaybackStatus::Playing {
            return;
        }
        let new_pos = g.state.position_ms.saturating_add(ms);
        if new_pos >= g.state.duration_ms {
            g.state.position_ms = g.state.duration_ms;
            g.state.status = PlaybackStatus::Stopped;
            let snapshot = g.state.clone();
            g.state.path = None;
            g.state.url = None;
            let _ = self.tx.send(AudioEvent::StateChanged(snapshot));
            let _ = self.tx.send(AudioEvent::TrackEnded);
        } else {
            g.state.position_ms = new_pos;
            let _ = self.tx.send(AudioEvent::Progress {
                position_ms: new_pos,
                duration_ms: g.state.duration_ms,
            });
        }
    }

    /// Passo de progresso usado pelo motor real, exposto para os testes.
    pub const PROGRESS_INTERVAL_MS: u64 = PROGRESS_INTERVAL_MS;

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn emit_state(&self, g: &Inner) {
        let _ = self.tx.send(AudioEvent::StateChanged(g.state.clone()));
    }
}

impl AudioBackend for NullBackend {
    // @req RF-01
    fn load(&self, path: &Path) -> Result<()> {
        let mut g = self.lock();
        if g.fail_loads {
            g.state.status = PlaybackStatus::Stopped;
            g.state.path = None;
            g.state.url = None;
            g.state.position_ms = 0;
            g.state.duration_ms = 0;
            self.emit_state(&g);
            let e = AudioError::Decode {
                path: path.to_path_buf(),
                message: "falha simulada".into(),
            };
            let _ = self.tx.send(AudioEvent::Error {
                message: e.to_string(),
            });
            return Err(e);
        }
        g.state.status = PlaybackStatus::Paused;
        g.state.path = Some(path.to_path_buf());
        g.state.url = None;
        g.state.position_ms = 0;
        g.state.duration_ms = g.next_duration_ms;
        self.emit_state(&g);
        Ok(())
    }

    // @req RF-18
    fn load_url(&self, url: &str, options: StreamOptions) -> Result<()> {
        let mut g = self.lock();
        if g.fail_loads {
            g.state.status = PlaybackStatus::Stopped;
            g.state.path = None;
            g.state.url = None;
            g.state.position_ms = 0;
            g.state.duration_ms = 0;
            self.emit_state(&g);
            let e = AudioError::Network {
                url: url.into(),
                message: "falha simulada".into(),
            };
            let _ = self.tx.send(AudioEvent::Error {
                message: e.to_string(),
            });
            return Err(e);
        }
        g.state.status = PlaybackStatus::Paused;
        g.state.path = None;
        g.state.url = Some(url.into());
        g.state.position_ms = 0;
        g.state.duration_ms = options.duration_ms;
        self.emit_state(&g);
        Ok(())
    }

    // @req RF-01
    fn play(&self) -> Result<()> {
        let mut g = self.lock();
        match g.state.status {
            PlaybackStatus::Playing => Ok(()),
            PlaybackStatus::Paused => {
                g.state.status = PlaybackStatus::Playing;
                self.emit_state(&g);
                Ok(())
            }
            _ => Err(AudioError::NothingLoaded),
        }
    }

    // @req RF-01
    fn pause(&self) -> Result<()> {
        let mut g = self.lock();
        if g.state.status == PlaybackStatus::Playing {
            g.state.status = PlaybackStatus::Paused;
            self.emit_state(&g);
        }
        Ok(())
    }

    // @req RF-01
    fn resume(&self) -> Result<()> {
        self.play()
    }

    // @req RF-01
    fn stop(&self) -> Result<()> {
        let mut g = self.lock();
        let changed = g.state.status != PlaybackStatus::Stopped;
        g.state.status = PlaybackStatus::Stopped;
        g.state.path = None;
        g.state.url = None;
        g.state.position_ms = 0;
        g.state.duration_ms = 0;
        if changed {
            self.emit_state(&g);
        }
        Ok(())
    }

    // @req RF-03
    fn seek(&self, position_ms: u64) -> Result<()> {
        let mut g = self.lock();
        if g.state.path.is_none() && g.state.url.is_none() {
            return Err(AudioError::NothingLoaded);
        }
        g.state.position_ms = position_ms.min(g.state.duration_ms);
        self.emit_state(&g);
        Ok(())
    }

    // @req RF-02
    fn set_volume(&self, volume: f32) -> Result<()> {
        let mut g = self.lock();
        g.state.volume = if volume.is_nan() {
            g.state.volume
        } else {
            volume.clamp(0.0, 1.0)
        };
        self.emit_state(&g);
        Ok(())
    }

    // @req RF-02
    fn set_muted(&self, muted: bool) -> Result<()> {
        let mut g = self.lock();
        g.state.muted = muted;
        self.emit_state(&g);
        Ok(())
    }

    fn state(&self) -> PlaybackState {
        self.lock().state.clone()
    }

    fn events(&self) -> Receiver<AudioEvent> {
        self.rx.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn p() -> PathBuf {
        PathBuf::from("/x/a.m4a")
    }

    fn drain(rx: &Receiver<AudioEvent>) -> Vec<AudioEvent> {
        rx.try_iter().collect()
    }

    // @test RF-01
    #[test]
    fn play_pause_resume_cycle() {
        let b = NullBackend::new();
        assert_eq!(b.state().status, PlaybackStatus::Stopped);
        assert!(matches!(b.play(), Err(AudioError::NothingLoaded)));
        b.load(&p()).unwrap();
        assert_eq!(b.state().status, PlaybackStatus::Paused);
        b.play().unwrap();
        assert_eq!(b.state().status, PlaybackStatus::Playing);
        b.advance(1000);
        b.pause().unwrap();
        b.advance(5000); // pausado: não anda
        assert_eq!(b.state().status, PlaybackStatus::Paused);
        assert_eq!(b.state().position_ms, 1000);
        b.resume().unwrap();
        b.advance(1000);
        assert_eq!(b.state().position_ms, 2000);
    }

    // @test RF-01
    #[test]
    fn natural_end_emits_track_ended_but_stop_does_not() {
        let b = NullBackend::new();
        let rx = b.events();
        b.set_next_duration_ms(2000);
        b.load(&p()).unwrap();
        b.play().unwrap();
        b.advance(1500);
        b.advance(1500);
        let ev = drain(&rx);
        assert_eq!(
            ev.iter().filter(|e| **e == AudioEvent::TrackEnded).count(),
            1
        );
        assert_eq!(b.state().status, PlaybackStatus::Stopped);
        // o StateChanged anterior ao fim ainda carrega a posição final
        assert!(ev.iter().any(|e| matches!(e,
            AudioEvent::StateChanged(s) if s.position_ms == 2000 && s.status == PlaybackStatus::Stopped)));

        b.load(&p()).unwrap();
        b.play().unwrap();
        b.stop().unwrap();
        b.load(&p()).unwrap(); // recarregar também não conta como fim
        assert!(!drain(&rx).contains(&AudioEvent::TrackEnded));
    }

    // @test RF-03
    #[test]
    fn seek_is_clamped_and_requires_track() {
        let b = NullBackend::new();
        assert!(matches!(b.seek(10), Err(AudioError::NothingLoaded)));
        b.set_next_duration_ms(10_000);
        b.load(&p()).unwrap();
        b.seek(4000).unwrap();
        assert_eq!(b.state().position_ms, 4000);
        b.seek(99_000).unwrap();
        assert_eq!(b.state().position_ms, 10_000);
    }

    // @test RF-02
    #[test]
    fn volume_is_clamped_and_mute_keeps_volume() {
        let b = NullBackend::new();
        b.set_volume(1.7).unwrap();
        assert_eq!(b.state().volume, 1.0);
        b.set_volume(-1.0).unwrap();
        assert_eq!(b.state().volume, 0.0);
        b.set_volume(0.4).unwrap();
        b.set_muted(true).unwrap();
        assert!(b.state().muted);
        assert_eq!(b.state().volume, 0.4);
        b.set_muted(false).unwrap();
        assert!(!b.state().muted);
    }

    // @test RF-02
    #[test]
    fn gain_curve_is_perceptual() {
        use crate::volume_to_gain;
        assert_eq!(volume_to_gain(0.0), 0.0);
        assert_eq!(volume_to_gain(1.0), 1.0);
        assert!(volume_to_gain(0.5) < 0.5);
        assert!(volume_to_gain(0.3) < volume_to_gain(0.6));
    }

    // @test RF-01
    #[test]
    fn failed_load_reports_error_and_keeps_state() {
        let b = NullBackend::new();
        b.set_fail_loads(true);
        assert!(matches!(b.load(&p()), Err(AudioError::Decode { .. })));
        assert_eq!(b.state().status, PlaybackStatus::Stopped);
    }

    // @test RF-01
    #[test]
    fn usable_as_trait_object() {
        let b: Box<dyn AudioBackend> = Box::new(NullBackend::new());
        b.load(&p()).unwrap();
        b.play().unwrap();
        assert_eq!(b.state().status, PlaybackStatus::Playing);
    }

    // @test RF-18
    #[test]
    fn stream_simulation_uses_known_duration_and_reports_failure() {
        let b = NullBackend::new();
        let rx = b.events();
        b.load_url(
            "https://example.test/tone.m4a",
            StreamOptions {
                headers: vec![("Referer".into(), "https://example.test".into())],
                duration_ms: 2000,
            },
        )
        .unwrap();
        assert_eq!(
            b.state().url.as_deref(),
            Some("https://example.test/tone.m4a")
        );
        assert_eq!(b.state().duration_ms, 2000);
        b.play().unwrap();
        b.seek(1500).unwrap();
        b.advance(500);
        assert!(drain(&rx).contains(&AudioEvent::TrackEnded));
        b.set_fail_loads(true);
        assert!(matches!(
            b.load_url("https://example.test/fail", StreamOptions::default()),
            Err(AudioError::Network { .. })
        ));
        assert!(
            drain(&rx)
                .iter()
                .any(|e| matches!(e, AudioEvent::Error { .. }))
        );
    }
}
