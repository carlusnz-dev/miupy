//! Backend desktop sobre `rodio` 0.23 (cpal + symphonia).
//!
//! Uma thread própria é dona do stream de saída e do `Player` (o stream do
//! cpal não é `Send` em todas as plataformas). O resto do programa fala com
//! ela por canal; o estado é espelhado num `Mutex` para consultas sem espera.
//! O callback de áudio é do rodio e não é tocado por este código.
//!
//! O dispositivo de saída só é aberto no primeiro `load`; sem dispositivo, o
//! `load` devolve [`AudioError::NoOutputDevice`].

use crate::error::{AudioError, Result};
use crate::types::{
    AudioBackend, AudioEvent, PROGRESS_INTERVAL_MS, PlaybackState, PlaybackStatus, volume_to_gain,
};
use crossbeam_channel::{Receiver, Sender, bounded, unbounded};
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player, Source};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

type Reply = Sender<Result<()>>;

enum Cmd {
    Load(PathBuf, Reply),
    Play(Reply),
    Pause(Reply),
    Stop(Reply),
    Seek(u64, Reply),
    SetVolume(f32, Reply),
    SetMuted(bool, Reply),
    /// Duração calculada em segundo plano (geração da faixa, ms).
    DurationResolved(u64, u64),
    Shutdown,
}

/// Backend de reprodução real para desktop.
pub struct RodioBackend {
    cmd_tx: Sender<Cmd>,
    shared: Arc<Mutex<PlaybackState>>,
    events_rx: Receiver<AudioEvent>,
    thread: Option<JoinHandle<()>>,
}

impl RodioBackend {
    /// Sobe a thread do motor. Não abre o dispositivo de áudio ainda.
    pub fn new() -> Result<Self> {
        let (cmd_tx, cmd_rx) = unbounded();
        let (ev_tx, events_rx) = unbounded();
        let shared = Arc::new(Mutex::new(PlaybackState::default()));
        let engine = Engine {
            sink: None,
            player: None,
            shared: Arc::clone(&shared),
            events: ev_tx,
            cmd_tx: cmd_tx.clone(),
            generation: 0,
            last_progress: Instant::now(),
        };
        let thread = std::thread::Builder::new()
            .name("miupy-audio".into())
            .spawn(move || engine.run(cmd_rx))
            .map_err(|e| AudioError::Unsupported(format!("thread do motor: {e}")))?;
        Ok(Self {
            cmd_tx,
            shared,
            events_rx,
            thread: Some(thread),
        })
    }

    fn call(&self, make: impl FnOnce(Reply) -> Cmd) -> Result<()> {
        let (tx, rx) = bounded(1);
        self.cmd_tx
            .send(make(tx))
            .map_err(|_| AudioError::EngineStopped)?;
        rx.recv().map_err(|_| AudioError::EngineStopped)?
    }
}

impl Drop for RodioBackend {
    fn drop(&mut self) {
        let _ = self.cmd_tx.send(Cmd::Shutdown);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl AudioBackend for RodioBackend {
    // @req RF-01
    fn load(&self, path: &Path) -> Result<()> {
        self.call(|r| Cmd::Load(path.to_path_buf(), r))
    }
    // @req RF-01
    fn play(&self) -> Result<()> {
        self.call(Cmd::Play)
    }
    // @req RF-01
    fn pause(&self) -> Result<()> {
        self.call(Cmd::Pause)
    }
    // @req RF-01
    fn resume(&self) -> Result<()> {
        self.call(Cmd::Play)
    }
    // @req RF-01
    fn stop(&self) -> Result<()> {
        self.call(Cmd::Stop)
    }
    // @req RF-03
    fn seek(&self, position_ms: u64) -> Result<()> {
        self.call(|r| Cmd::Seek(position_ms, r))
    }
    // @req RF-02
    fn set_volume(&self, volume: f32) -> Result<()> {
        self.call(|r| Cmd::SetVolume(volume, r))
    }
    // @req RF-02
    fn set_muted(&self, muted: bool) -> Result<()> {
        self.call(|r| Cmd::SetMuted(muted, r))
    }
    fn state(&self) -> PlaybackState {
        self.shared
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
    fn events(&self) -> Receiver<AudioEvent> {
        self.events_rx.clone()
    }
}

struct Engine {
    sink: Option<MixerDeviceSink>,
    player: Option<Player>,
    shared: Arc<Mutex<PlaybackState>>,
    events: Sender<AudioEvent>,
    cmd_tx: Sender<Cmd>,
    /// Incrementa a cada load/stop, para descartar resultados de faixas antigas.
    generation: u64,
    last_progress: Instant,
}

impl Engine {
    fn run(mut self, rx: Receiver<Cmd>) {
        let tick = Duration::from_millis(100);
        loop {
            match rx.recv_timeout(tick) {
                Ok(Cmd::Shutdown) => break,
                Ok(cmd) => self.handle(cmd),
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
            }
            self.tick();
        }
        if let Some(p) = self.player.take() {
            p.stop();
        }
    }

    fn st(&self) -> std::sync::MutexGuard<'_, PlaybackState> {
        self.shared.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn emit_state(&self) {
        let s = self.st().clone();
        let _ = self.events.send(AudioEvent::StateChanged(s));
    }

    fn apply_gain(&self) {
        if let Some(p) = &self.player {
            let s = self.st();
            p.set_volume(if s.muted {
                0.0
            } else {
                volume_to_gain(s.volume)
            });
        }
    }

    fn handle(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::Load(path, r) => {
                let _ = r.send(self.load(&path));
            }
            Cmd::Play(r) => {
                let _ = r.send(self.play());
            }
            Cmd::Pause(r) => {
                if let Some(p) = &self.player {
                    let playing = self.st().status == PlaybackStatus::Playing;
                    if playing {
                        p.pause();
                        self.st().position_ms = p.get_pos().as_millis() as u64;
                        self.st().status = PlaybackStatus::Paused;
                        self.emit_state();
                    }
                }
                let _ = r.send(Ok(()));
            }
            Cmd::Stop(r) => {
                self.unload();
                let _ = r.send(Ok(()));
            }
            Cmd::Seek(ms, r) => {
                let _ = r.send(self.seek(ms));
            }
            Cmd::SetVolume(v, r) => {
                if !v.is_nan() {
                    self.st().volume = v.clamp(0.0, 1.0);
                }
                self.apply_gain();
                self.emit_state();
                let _ = r.send(Ok(()));
            }
            Cmd::SetMuted(m, r) => {
                self.st().muted = m;
                self.apply_gain();
                self.emit_state();
                let _ = r.send(Ok(()));
            }
            Cmd::DurationResolved(generation, ms) => {
                if generation == self.generation && self.player.is_some() {
                    self.st().duration_ms = ms;
                    self.emit_state();
                }
            }
            Cmd::Shutdown => {}
        }
    }

    /// Descarta a faixa atual sem emitir `TrackEnded`.
    fn unload(&mut self) {
        self.generation += 1;
        if let Some(p) = self.player.take() {
            p.stop();
        }
        let changed = {
            let mut s = self.st();
            let changed = s.status != PlaybackStatus::Stopped || s.path.is_some();
            s.status = PlaybackStatus::Stopped;
            s.path = None;
            s.position_ms = 0;
            s.duration_ms = 0;
            changed
        };
        if changed {
            self.emit_state();
        }
    }

    fn ensure_sink(&mut self) -> Result<&MixerDeviceSink> {
        if self.sink.is_none() {
            let mut sink = DeviceSinkBuilder::open_default_sink()
                .map_err(|e| AudioError::NoOutputDevice(e.to_string()))?;
            sink.log_on_drop(false);
            self.sink = Some(sink);
        }
        Ok(self.sink.as_ref().expect("sink acabou de ser criado"))
    }

    fn load(&mut self, path: &Path) -> Result<()> {
        self.unload();
        self.st().status = PlaybackStatus::Loading;
        self.st().path = Some(path.to_path_buf());
        self.emit_state();
        match self.load_inner(path) {
            Ok(()) => Ok(()),
            Err(e) => {
                self.unload();
                Err(e)
            }
        }
    }

    fn load_inner(&mut self, path: &Path) -> Result<()> {
        self.ensure_sink()?;
        let file = File::open(path).map_err(|source| AudioError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let len = file
            .metadata()
            .map_err(|source| AudioError::Io {
                path: path.to_path_buf(),
                source,
            })?
            .len();
        let mut builder = Decoder::builder()
            .with_data(BufReader::new(file))
            .with_byte_len(len)
            .with_seekable(true);
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            builder = builder.with_hint(ext);
        }
        let decoder = builder.build().map_err(|e| AudioError::Decode {
            path: path.to_path_buf(),
            message: e.to_string(),
        })?;
        let known = decoder.total_duration().map(|d| d.as_millis() as u64);

        let mixer = self.sink.as_ref().expect("sink aberto").mixer();
        let player = Player::connect_new(mixer);
        player.pause();
        player.append(decoder);
        self.player = Some(player);
        {
            let mut s = self.st();
            s.status = PlaybackStatus::Paused;
            s.position_ms = 0;
            s.duration_ms = known.unwrap_or(0);
        }
        self.apply_gain();
        self.emit_state();

        // M4A/MP3 nem sempre informam a duração: calcula em segundo plano.
        if known.is_none() {
            let generation = self.generation;
            let tx = self.cmd_tx.clone();
            let path = path.to_path_buf();
            let _ = std::thread::Builder::new()
                .name("miupy-audio-probe".into())
                .spawn(move || {
                    if let Some(d) = probe_duration(&path) {
                        let _ = tx.send(Cmd::DurationResolved(generation, d.as_millis() as u64));
                    }
                });
        }
        Ok(())
    }

    fn play(&mut self) -> Result<()> {
        let Some(p) = &self.player else {
            return Err(AudioError::NothingLoaded);
        };
        if self.st().status == PlaybackStatus::Paused {
            p.play();
            self.st().status = PlaybackStatus::Playing;
            self.last_progress = Instant::now();
            self.emit_state();
        }
        Ok(())
    }

    fn seek(&mut self, ms: u64) -> Result<()> {
        let Some(p) = &self.player else {
            return Err(AudioError::NothingLoaded);
        };
        let duration = self.st().duration_ms;
        let target = if duration > 0 { ms.min(duration) } else { ms };
        p.try_seek(Duration::from_millis(target))
            .map_err(|e| AudioError::Seek(e.to_string()))?;
        self.st().position_ms = target;
        self.emit_state();
        Ok(())
    }

    /// Detecta fim natural e publica o progresso periódico.
    fn tick(&mut self) {
        let Some(p) = &self.player else { return };
        if self.st().status != PlaybackStatus::Playing {
            return;
        }
        if p.empty() {
            // @req RF-01
            self.player = None;
            self.generation += 1;
            {
                let mut s = self.st();
                s.position_ms = s.duration_ms;
                s.status = PlaybackStatus::Stopped;
            }
            self.emit_state();
            self.st().path = None;
            let _ = self.events.send(AudioEvent::TrackEnded);
            return;
        }
        let pos = p.get_pos().as_millis() as u64;
        let duration = {
            let mut s = self.st();
            s.position_ms = if s.duration_ms > 0 {
                pos.min(s.duration_ms)
            } else {
                pos
            };
            s.duration_ms
        };
        if self.last_progress.elapsed() >= Duration::from_millis(PROGRESS_INTERVAL_MS) {
            self.last_progress = Instant::now();
            let _ = self.events.send(AudioEvent::Progress {
                position_ms: self.st().position_ms,
                duration_ms: duration,
            });
        }
    }
}

/// Duração exata, contando amostras decodificadas (fallback quando o
/// container não informa). Custa decodificar o arquivo inteiro uma vez.
fn probe_duration(path: &Path) -> Option<Duration> {
    let file = File::open(path).ok()?;
    let len = file.metadata().ok()?.len();
    let mut b = Decoder::builder()
        .with_data(BufReader::new(file))
        .with_byte_len(len)
        .with_seekable(true);
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        b = b.with_hint(ext);
    }
    let decoder = b.build().ok()?;
    let channels = u64::from(decoder.channels().get());
    let rate = u64::from(decoder.sample_rate().get());
    let samples = decoder.count() as u64;
    if rate == 0 || channels == 0 || samples == 0 {
        return None;
    }
    Some(Duration::from_micros(
        samples * 1_000_000 / (channels * rate),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn write_wav(name: &str, secs: f32) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("miupy-audio-{}-{name}.wav", std::process::id()));
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 44_100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut w = hound::WavWriter::create(&path, spec).unwrap();
        for i in 0..(44_100.0 * secs) as usize {
            let s = (i as f32 * 440.0 * std::f32::consts::TAU / 44_100.0).sin();
            w.write_sample((s * 8000.0) as i16).unwrap();
        }
        w.finalize().unwrap();
        path
    }

    /// Cria o backend e carrega o WAV; devolve `None` (teste pulado) sem dispositivo.
    fn backend_or_skip(path: &Path) -> Option<RodioBackend> {
        let b = RodioBackend::new().unwrap();
        match b.load(path) {
            Ok(()) => {
                b.set_muted(true).unwrap(); // nada de barulho nos testes
                Some(b)
            }
            Err(AudioError::NoOutputDevice(why)) => {
                eprintln!("PULADO: sem dispositivo de saída de áudio ({why})");
                None
            }
            Err(e) => panic!("load falhou: {e}"),
        }
    }

    fn wait_for(rx: &Receiver<AudioEvent>, secs: u64, f: impl Fn(&AudioEvent) -> bool) -> bool {
        let deadline = Instant::now() + Duration::from_secs(secs);
        while let Some(left) = deadline.checked_duration_since(Instant::now()) {
            match rx.recv_timeout(left) {
                Ok(e) if f(&e) => return true,
                Ok(_) => {}
                Err(_) => return false,
            }
        }
        false
    }

    // @test RF-01
    #[test]
    fn load_reports_duration_and_plays_to_natural_end() {
        let path = write_wav("end", 1.0);
        let Some(b) = backend_or_skip(&path) else {
            return;
        };
        let s = b.state();
        assert_eq!(s.status, PlaybackStatus::Paused);
        assert!(
            (900..=1100).contains(&s.duration_ms),
            "duração {}",
            s.duration_ms
        );
        let rx = b.events();
        b.play().unwrap();
        assert_eq!(b.state().status, PlaybackStatus::Playing);
        assert!(
            wait_for(&rx, 5, |e| *e == AudioEvent::TrackEnded),
            "sem TrackEnded"
        );
        assert_eq!(b.state().status, PlaybackStatus::Stopped);
        std::fs::remove_file(path).ok();
    }

    // @test RF-01
    #[test]
    fn pause_holds_position_and_stop_is_not_track_end() {
        let path = write_wav("pause", 3.0);
        let Some(b) = backend_or_skip(&path) else {
            return;
        };
        let rx = b.events();
        b.play().unwrap();
        std::thread::sleep(Duration::from_millis(600));
        b.pause().unwrap();
        let p1 = b.state().position_ms;
        assert!(p1 > 200, "posição {p1}");
        std::thread::sleep(Duration::from_millis(400));
        assert_eq!(b.state().position_ms, p1);
        b.resume().unwrap();
        b.stop().unwrap();
        assert_eq!(b.state().status, PlaybackStatus::Stopped);
        std::thread::sleep(Duration::from_millis(300));
        assert!(!rx.try_iter().any(|e| e == AudioEvent::TrackEnded));
        std::fs::remove_file(path).ok();
    }

    // @test RF-03
    #[test]
    fn seek_moves_position() {
        let path = write_wav("seek", 4.0);
        let Some(b) = backend_or_skip(&path) else {
            return;
        };
        b.play().unwrap();
        b.seek(2500).unwrap();
        let p = b.state().position_ms;
        assert!((2400..=2700).contains(&p), "posição {p}");
        b.seek(999_999).unwrap(); // limitado à duração
        assert!(b.state().position_ms <= b.state().duration_ms);
        std::fs::remove_file(path).ok();
    }

    // @test RF-02
    #[test]
    fn volume_and_mute_update_state() {
        let path = write_wav("vol", 1.0);
        let Some(b) = backend_or_skip(&path) else {
            return;
        };
        b.set_volume(0.3).unwrap();
        b.set_muted(false).unwrap();
        let s = b.state();
        assert_eq!(s.volume, 0.3);
        assert!(!s.muted);
        std::fs::remove_file(path).ok();
    }

    // @test RF-01
    #[test]
    fn missing_file_is_a_clear_error() {
        let b = RodioBackend::new().unwrap();
        match b.load(Path::new("/nao/existe.m4a")) {
            Err(AudioError::Io { .. }) | Err(AudioError::NoOutputDevice(_)) => {}
            other => panic!("esperava Io ou NoOutputDevice, veio {other:?}"),
        }
        assert_eq!(b.state().status, PlaybackStatus::Stopped);
    }

    // @test RF-01
    #[test]
    fn fallback_probe_counts_samples() {
        let path = write_wav("probe", 1.5);
        let d = probe_duration(&path).unwrap().as_millis();
        assert!((1450..=1550).contains(&d), "duração {d}");
        std::fs::remove_file(path).ok();
    }
}
