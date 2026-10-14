//! Backend desktop sobre `rodio` 0.23 (cpal + symphonia).
//!
//! Uma thread própria é dona do stream de saída e do `Player` (o stream do
//! cpal não é `Send` em todas as plataformas). O resto do programa fala com
//! ela por canal; o estado é espelhado num `Mutex` para consultas sem espera.
//! O callback de áudio é do rodio e não é tocado por este código.
//!
//! O dispositivo de saída só é aberto no primeiro `load`; sem dispositivo, o
//! `load` devolve [`AudioError::NoOutputDevice`].

use crate::dsp::{DspControls, DspSource};
use crate::error::{AudioError, Result};
use crate::types::{
    AudioBackend, AudioEvent, PROGRESS_INTERVAL_MS, PlaybackState, PlaybackStatus, StreamOptions,
    volume_to_gain,
};
use crossbeam_channel::{Receiver, Sender, bounded, unbounded};
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player, Source};
use std::fs::File;
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use stream_download::http::{HttpStream, reqwest};
use stream_download::storage::memory::MemoryStorageProvider;
use stream_download::{Settings, StreamDownload};

type HttpReader = StreamDownload<MemoryStorageProvider>;
type HttpDecoder = Decoder<BufReader<FailureReader<HttpReader>>>;

type Reply = Sender<Result<()>>;

enum Cmd {
    Load(PathBuf, Reply),
    LoadUrl(String, StreamOptions, Reply),
    StreamReady(u64, HttpDecoder, Arc<Mutex<Option<String>>>),
    StreamFailed(u64, String),
    OutputError(String),
    Play(Reply),
    Pause(Reply),
    Stop(Reply),
    Seek(u64, Reply),
    SetVolume(f32, Reply),
    SetNormalizationGain(f32, Reply),
    SetEqualizer([f32; 8], Reply),
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
            dsp: Arc::new(DspControls::default()),
            player: None,
            shared: Arc::clone(&shared),
            events: ev_tx,
            cmd_tx: cmd_tx.clone(),
            generation: 0,
            last_progress: Instant::now(),
            stream_failure: None,
            play_when_ready: false,
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
    // @req RF-18
    fn load_url(&self, url: &str, options: StreamOptions) -> Result<()> {
        self.call(|r| Cmd::LoadUrl(url.into(), options, r))
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
    // @req RF-28
    fn set_normalization_gain_db(&self, gain_db: f32) -> Result<()> {
        self.call(|r| Cmd::SetNormalizationGain(gain_db, r))
    }
    // @req RF-29
    fn set_equalizer(&self, gains_db: [f32; 8]) -> Result<()> {
        self.call(|r| Cmd::SetEqualizer(gains_db, r))
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
    dsp: Arc<DspControls>,
    sink: Option<MixerDeviceSink>,
    player: Option<Player>,
    shared: Arc<Mutex<PlaybackState>>,
    events: Sender<AudioEvent>,
    cmd_tx: Sender<Cmd>,
    /// Incrementa a cada load/stop, para descartar resultados de faixas antigas.
    generation: u64,
    last_progress: Instant,
    stream_failure: Option<Arc<Mutex<Option<String>>>>,
    play_when_ready: bool,
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
        let s = self.st();
        let gain = if s.muted {
            0.0
        } else {
            volume_to_gain(s.volume)
        };
        self.dsp.volume.store(gain.to_bits(), Ordering::Relaxed);
    }

    fn handle(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::Load(path, r) => {
                let _ = r.send(self.load(&path));
            }
            Cmd::LoadUrl(url, options, r) => {
                let _ = r.send(self.load_url(url, options));
            }
            Cmd::StreamReady(generation, decoder, failure) => {
                if generation == self.generation {
                    let pending_seek = self.st().position_ms;
                    match self.ensure_sink() {
                        Ok(_) => {
                            let player = Player::connect_new(
                                self.sink.as_ref().expect("sink aberto").mixer(),
                            );
                            player.pause();
                            player.append(DspSource::new(decoder, Arc::clone(&self.dsp)));
                            self.player = Some(player);
                            self.stream_failure = Some(failure);
                            self.st().status = PlaybackStatus::Paused;
                            self.apply_gain();
                            self.emit_state();
                            if pending_seek > 0
                                && let Err(e) = self.seek(pending_seek)
                            {
                                self.fail(e.to_string());
                                return;
                            }
                            if self.play_when_ready {
                                let _ = self.play();
                            }
                        }
                        Err(e) => self.fail(e.to_string()),
                    }
                }
            }
            Cmd::StreamFailed(generation, message) => {
                if generation == self.generation {
                    self.fail(message);
                }
            }
            Cmd::OutputError(message) => {
                // Underrun/overrun é um engasgo momentâneo do dispositivo (acontece, por
                // exemplo, enquanto um seek espera dados); o stream de saída continua válido.
                let lower = message.to_lowercase();
                if lower.contains("underrun") || lower.contains("overrun") {
                    return;
                }
                self.sink = None;
                self.fail(format!("dispositivo de saída: {message}"));
            }
            Cmd::Play(r) => {
                let _ = r.send(self.play());
            }
            Cmd::Pause(r) => {
                if self.st().status == PlaybackStatus::Loading {
                    self.play_when_ready = false;
                }
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
            Cmd::SetNormalizationGain(db, r) => {
                if db.is_finite() {
                    let gain = 10.0_f32.powf(db.clamp(-20.0, 12.0) / 20.0);
                    self.dsp
                        .normalization
                        .store(gain.to_bits(), Ordering::Relaxed);
                }
                let _ = r.send(Ok(()));
            }
            Cmd::SetEqualizer(gains, r) => {
                self.dsp.set_equalizer(gains);
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
        self.stream_failure = None;
        self.play_when_ready = false;
        let changed = {
            let mut s = self.st();
            let changed =
                s.status != PlaybackStatus::Stopped || s.path.is_some() || s.url.is_some();
            s.status = PlaybackStatus::Stopped;
            s.path = None;
            s.url = None;
            s.position_ms = 0;
            s.duration_ms = 0;
            changed
        };
        if changed {
            self.emit_state();
        }
    }

    fn fail(&mut self, message: String) {
        self.unload();
        let _ = self.events.send(AudioEvent::Error { message });
    }

    fn ensure_sink(&mut self) -> Result<&MixerDeviceSink> {
        if self.sink.is_none() {
            let tx = self.cmd_tx.clone();
            let mut sink = DeviceSinkBuilder::from_default_device()
                .map_err(|e| AudioError::NoOutputDevice(e.to_string()))?
                .with_error_callback(move |e| {
                    let _ = tx.send(Cmd::OutputError(e.to_string()));
                })
                .open_stream()
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
                self.fail(e.to_string());
                Err(e)
            }
        }
    }

    fn load_inner(&mut self, path: &Path) -> Result<()> {
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
        let decoder = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| builder.build()))
            .map_err(|_| AudioError::Decode {
                path: path.to_path_buf(),
                message: "falha interna do decodificador".into(),
            })?
            .map_err(|e| AudioError::Decode {
                path: path.to_path_buf(),
                message: e.to_string(),
            })?;
        let known = decoder.total_duration().map(|d| d.as_millis() as u64);

        self.ensure_sink()?;
        let mixer = self.sink.as_ref().expect("sink aberto").mixer();
        let player = Player::connect_new(mixer);
        player.pause();
        player.append(DspSource::new(decoder, Arc::clone(&self.dsp)));
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

    // @req RF-18
    fn load_url(&mut self, url: String, options: StreamOptions) -> Result<()> {
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            let e = AudioError::Network {
                url,
                message: "URL HTTP inválida".into(),
            };
            self.fail(e.to_string());
            return Err(e);
        }
        self.unload();
        self.st().status = PlaybackStatus::Loading;
        self.st().url = Some(url.clone());
        self.st().duration_ms = options.duration_ms;
        self.emit_state();
        let generation = self.generation;
        let tx = self.cmd_tx.clone();
        std::thread::Builder::new()
            .name("miupy-audio-http".into())
            .spawn(move || {
                if let Err(message) = open_stream(url, options, generation, tx.clone()) {
                    let _ = tx.send(Cmd::StreamFailed(generation, message));
                }
            })
            .map_err(|e| {
                let err = AudioError::Network {
                    url: self.st().url.clone().unwrap_or_default(),
                    message: e.to_string(),
                };
                self.fail(err.to_string());
                err
            })?;
        Ok(())
    }

    fn play(&mut self) -> Result<()> {
        let Some(p) = &self.player else {
            if self.st().status == PlaybackStatus::Loading {
                self.play_when_ready = true;
                return Ok(());
            }
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
            if self.st().status == PlaybackStatus::Loading {
                self.st().position_ms = ms.min(self.st().duration_ms);
                self.emit_state();
                return Ok(());
            }
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
        if let Some(failure) = &self.stream_failure {
            let message = failure.lock().unwrap_or_else(|e| e.into_inner()).take();
            if let Some(message) = message {
                self.fail(message);
                return;
            }
        }
        let Some(p) = &self.player else { return };
        if self.st().status != PlaybackStatus::Playing {
            return;
        }
        if p.empty() {
            let pos = p.get_pos().as_millis() as u64;
            let duration = self.st().duration_ms;
            if duration > 0 && pos.saturating_add(250) < duration {
                self.fail(format!(
                    "decodificação interrompida em {pos} ms de {duration} ms"
                ));
                return;
            }
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
            self.st().url = None;
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

/// Tempo máximo que o decoder pode ficar esperando bytes da rede antes de a faixa ser dada
/// como interrompida. Mede a espera de quem toca, não o andamento do download: uma faixa que
/// já chegou inteira não baixa mais nada e isso não é falha.
const STREAM_STALL_TIMEOUT: Duration = Duration::from_secs(8);

/// Prazo para a abertura. O decoder busca por bytes (é o que torna o seek imediato) e, para
/// isso, o Symphonia percorre os átomos do MP4 até o fim: a faixa só fica pronta quando o
/// arquivo inteiro chegou. Um M4A de alguns MB leva de 1 a 3 s numa conexão comum; o prazo
/// cobre conexões lentas.
#[cfg(not(test))]
const STREAM_OPEN_TIMEOUT: Duration = Duration::from_secs(30);
#[cfg(test)]
const STREAM_OPEN_TIMEOUT: Duration = Duration::from_secs(6);

/// Propaga erros de leitura que o decoder do rodio converte em fim de iterador e registra
/// desde quando uma leitura ou busca está bloqueada esperando a rede.
struct FailureReader<R> {
    inner: R,
    failure: Arc<Mutex<Option<String>>>,
    waiting_since: Arc<Mutex<Option<Instant>>>,
}

impl<R> FailureReader<R> {
    fn track<T>(&mut self, op: impl FnOnce(&mut R) -> io::Result<T>) -> io::Result<T> {
        *self.waiting_since.lock().unwrap_or_else(|p| p.into_inner()) = Some(Instant::now());
        let result = op(&mut self.inner);
        *self.waiting_since.lock().unwrap_or_else(|p| p.into_inner()) = None;
        result.inspect_err(|e| {
            *self.failure.lock().unwrap_or_else(|p| p.into_inner()) = Some(e.to_string());
        })
    }
}

impl<R: Read> Read for FailureReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let limit = buf.len().min(4096);
        self.track(|inner| inner.read(&mut buf[..limit]))
    }
}

impl<R: Seek> Seek for FailureReader<R> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.track(|inner| inner.seek(pos))
    }
}

// @req RF-18
fn open_stream(
    url: String,
    options: StreamOptions,
    generation: u64,
    tx: Sender<Cmd>,
) -> std::result::Result<(), String> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    let parsed = reqwest::Url::parse(&url).map_err(|e| e.to_string())?;
    let mut headers = reqwest::header::HeaderMap::new();
    for (name, value) in &options.headers {
        let name =
            reqwest::header::HeaderName::from_bytes(name.as_bytes()).map_err(|e| e.to_string())?;
        let value = reqwest::header::HeaderValue::from_str(value).map_err(|e| e.to_string())?;
        headers.insert(name, value);
    }
    let client = reqwest::Client::builder()
        .default_headers(headers)
        // Sem prazo total: o corpo de uma faixa longa leva mais que alguns segundos para
        // chegar. O que não pode é a conexão ou uma leitura ficarem paradas.
        .connect_timeout(Duration::from_secs(10))
        .read_timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;
    let stream = runtime
        .block_on(HttpStream::new(client, parsed))
        .map_err(|e| e.to_string())?;
    let content_length = stream_download::source::SourceStream::content_length(&stream);
    let settings: Settings<HttpStream<reqwest::Client>> = Settings::default()
        .prefetch_bytes(8 * 1024)
        .retry_timeout(Duration::from_secs(3));
    let reader = runtime
        .block_on(StreamDownload::from_stream(
            stream,
            MemoryStorageProvider,
            settings,
        ))
        .map_err(|e| e.to_string())?;
    let cancellation = reader.cancellation_token();
    let watch_cancel = cancellation.clone();
    let waiting_since = Arc::new(Mutex::new(None::<Instant>));
    let watch_waiting = Arc::clone(&waiting_since);
    let opened = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let watch_opened = Arc::clone(&opened);
    let open_started = Instant::now();
    let watch_tx = tx.clone();
    let watch_url = url.clone();
    runtime.spawn(async move {
        loop {
            if watch_cancel.is_cancelled() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
            let stalled = if watch_opened.load(std::sync::atomic::Ordering::Relaxed) {
                watch_waiting
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .is_some_and(|since| since.elapsed() > STREAM_STALL_TIMEOUT)
            } else {
                open_started.elapsed() > STREAM_OPEN_TIMEOUT
            };
            if stalled {
                let _ = watch_tx.send(Cmd::StreamFailed(
                    generation,
                    format!("rede interrompida ao tocar {watch_url}"),
                ));
                watch_cancel.cancel();
                break;
            }
        }
    });
    let failure = Arc::new(Mutex::new(None));
    let tracked = FailureReader {
        inner: reader,
        failure: Arc::clone(&failure),
        waiting_since,
    };
    // Com o tamanho conhecido o decoder busca por bytes (o leitor baixa o trecho pedido);
    // sem ele, um seek para frente decodificaria tudo até o ponto, o que leva vários segundos.
    let mut builder = Decoder::builder().with_data(BufReader::new(tracked));
    builder = match content_length {
        Some(len) if len > 0 => builder.with_byte_len(len).with_seekable(true),
        _ => builder.with_seekable(false),
    };
    if let Some(ext) = url.split('?').next().and_then(|s| s.rsplit('.').next())
        && ext.len() <= 5
    {
        builder = builder.with_hint(ext);
    }
    let decoder = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| builder.build()))
        .map_err(|_| "falha interna do decodificador".to_string())?
        .map_err(|e| e.to_string())?;
    opened.store(true, std::sync::atomic::Ordering::Relaxed);
    tx.send(Cmd::StreamReady(generation, decoder, failure))
        .map_err(|e| e.to_string())?;
    runtime.block_on(cancellation.cancelled());
    Ok(())
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
    use std::io::Write;
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    type TestServer = (
        String,
        Arc<AtomicBool>,
        Arc<Mutex<Vec<String>>>,
        JoinHandle<()>,
    );

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
    }

    fn fixture_decoder(path: &Path) -> Decoder<BufReader<File>> {
        let file = File::open(path).unwrap();
        let len = file.metadata().unwrap().len();
        Decoder::builder()
            .with_data(BufReader::new(file))
            .with_byte_len(len)
            .with_seekable(true)
            .with_hint(path.extension().unwrap().to_str().unwrap())
            .build()
            .unwrap()
    }

    // @test RF-01 RF-03
    #[test]
    fn real_formats_decode_and_seek_without_output() {
        for name in [
            "tone.m4a",
            "tone-no-duration.m4a",
            "tone-fragmented.m4a",
            "tone.mp3",
        ] {
            let path = fixture(name);
            let mut decoder = fixture_decoder(&path);
            assert!(decoder.next().is_some(), "sem amostras: {name}");
            decoder.try_seek(Duration::from_millis(1000)).unwrap();
            assert!(decoder.next().is_some(), "seek sem amostras: {name}");
            let duration = probe_duration(&path).unwrap().as_millis();
            assert!((1900..=2150).contains(&duration), "{name}: {duration} ms");
        }
        let no_metadata = fixture_decoder(&fixture("tone-no-duration.m4a"));
        assert!(no_metadata.total_duration().is_none());
    }

    // @test RF-01 RF-03
    #[test]
    fn real_formats_play_pause_seek_and_end_when_output_exists() {
        for name in [
            "tone.m4a",
            "tone-no-duration.m4a",
            "tone-fragmented.m4a",
            "tone.mp3",
        ] {
            let Some(b) = backend_or_skip(&fixture(name)) else {
                return;
            };
            let duration_deadline = Instant::now() + Duration::from_secs(3);
            while b.state().duration_ms == 0 && Instant::now() < duration_deadline {
                std::thread::sleep(Duration::from_millis(10));
            }
            assert!(
                (1900..=2150).contains(&b.state().duration_ms),
                "duração de {name}: {}",
                b.state().duration_ms
            );
            let rx = b.events();
            b.play().unwrap();
            std::thread::sleep(Duration::from_millis(150));
            b.pause().unwrap();
            assert_eq!(b.state().status, PlaybackStatus::Paused, "{name}");
            b.seek(1200).unwrap();
            assert!((1100..=1300).contains(&b.state().position_ms), "{name}");
            b.resume().unwrap();
            assert!(
                wait_for(&rx, 5, |e| *e == AudioEvent::TrackEnded),
                "sem fim: {name}"
            );
        }
    }

    // @test RF-01
    #[test]
    fn corrupt_and_missing_files_emit_error_and_allow_new_load() {
        let b = RodioBackend::new().unwrap();
        let rx = b.events();
        let missing = Path::new("/nao/existe/miupy.m4a");
        assert!(matches!(b.load(missing), Err(AudioError::Io { .. })));
        assert!(rx.try_iter().any(|e| matches!(e, AudioEvent::Error { .. })));
        for bytes in [
            b"not audio".as_slice(),
            &std::fs::read(fixture("tone.m4a")).unwrap()[..1000],
        ] {
            let path = std::env::temp_dir().join(format!("miupy-bad-{}.m4a", std::process::id()));
            std::fs::write(&path, bytes).unwrap();
            assert!(matches!(b.load(&path), Err(AudioError::Decode { .. })));
            assert_eq!(b.state().status, PlaybackStatus::Stopped);
            assert!(rx.try_iter().any(|e| matches!(e, AudioEvent::Error { .. })));
            std::fs::remove_file(path).unwrap();
        }
        match b.load(&fixture("tone.m4a")) {
            Ok(()) => assert_eq!(b.state().status, PlaybackStatus::Paused),
            Err(AudioError::NoOutputDevice(why)) => {
                eprintln!("PULADO: recuperação com saída real ({why})")
            }
            Err(e) => panic!("recarga falhou: {e}"),
        }
    }

    // @test RF-01
    #[test]
    fn truncated_audio_during_playback_emits_error() {
        let bytes = std::fs::read(fixture("tone.mp3")).unwrap();
        let path = std::env::temp_dir().join(format!("miupy-truncated-{}.mp3", std::process::id()));
        std::fs::write(&path, &bytes[..bytes.len() / 2]).unwrap();
        let b = RodioBackend::new().unwrap();
        let rx = b.events();
        match b.load(&path) {
            Ok(()) => {
                b.set_muted(true).unwrap();
                b.play().unwrap();
                assert!(
                    wait_for(&rx, 5, |e| matches!(e, AudioEvent::Error { .. })),
                    "arquivo truncado terminou sem erro"
                );
                assert_eq!(b.state().status, PlaybackStatus::Stopped);
                assert!(!rx.try_iter().any(|e| e == AudioEvent::TrackEnded));
            }
            Err(AudioError::Decode { .. }) => {
                assert!(rx.try_iter().any(|e| matches!(e, AudioEvent::Error { .. })));
            }
            Err(AudioError::NoOutputDevice(why)) => {
                eprintln!("PULADO: truncamento em reprodução ({why})")
            }
            Err(e) => panic!("erro inesperado: {e}"),
        }
        std::fs::remove_file(path).unwrap();
    }

    // @test RF-01
    #[test]
    fn output_failure_emits_error_and_backend_recovers() {
        let b = RodioBackend::new().unwrap();
        match b.load(&fixture("tone.m4a")) {
            Ok(()) => {}
            Err(AudioError::NoOutputDevice(why)) => {
                eprintln!("PULADO: falha de dispositivo ({why})");
                return;
            }
            Err(e) => panic!("load falhou: {e}"),
        }
        let rx = b.events();
        b.cmd_tx
            .send(Cmd::OutputError("falha simulada".into()))
            .unwrap();
        assert!(wait_for(
            &rx,
            2,
            |e| matches!(e, AudioEvent::Error { message } if message.contains("falha simulada"))
        ));
        assert_eq!(b.state().status, PlaybackStatus::Stopped);
        b.load(&fixture("tone.m4a")).unwrap();
        assert_eq!(b.state().status, PlaybackStatus::Paused);
    }

    fn serve_fixture(name: &str, drop_body: bool, pause_ms: u64, tail_bytes: usize) -> TestServer {
        let bytes = Arc::new(std::fs::read(fixture(name)).unwrap());
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}/tone.{name}", listener.local_addr().unwrap());
        let stop = Arc::new(AtomicBool::new(false));
        let log = Arc::new(Mutex::new(Vec::new()));
        let stop_worker = Arc::clone(&stop);
        let log_worker = Arc::clone(&log);
        let thread = std::thread::spawn(move || {
            let mut request_count = 0;
            while !stop_worker.load(Ordering::Relaxed) {
                let Ok((mut socket, _)) = listener.accept() else {
                    std::thread::sleep(Duration::from_millis(10));
                    continue;
                };
                socket
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut request = Vec::new();
                let mut chunk = [0; 1024];
                while !request.ends_with(b"\r\n\r\n") && request.len() < 8192 {
                    let Ok(n) = socket.read(&mut chunk) else {
                        break;
                    };
                    if n == 0 {
                        break;
                    }
                    request.extend_from_slice(&chunk[..n]);
                }
                let request = String::from_utf8_lossy(&request).into_owned();
                log_worker.lock().unwrap().push(request.clone());
                request_count += 1;
                if drop_body && request_count > 1 {
                    continue;
                }
                let range = request.lines().find_map(|line| {
                    line.strip_prefix("range: bytes=")
                        .or_else(|| line.strip_prefix("Range: bytes="))
                });
                let (start, end) = range
                    .and_then(|value| {
                        let (a, b) = value.split_once('-')?;
                        let start = a.parse::<usize>().ok()?;
                        let end = b
                            .parse::<usize>()
                            .ok()
                            .map(|v| v + 1)
                            .unwrap_or(bytes.len())
                            .min(bytes.len());
                        Some((start, end))
                    })
                    .unwrap_or((0, bytes.len()));
                if start >= bytes.len() {
                    continue;
                }
                let body = &bytes[start..end];
                let status = if range.is_some() {
                    "206 Partial Content"
                } else {
                    "200 OK"
                };
                let header = format!(
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nAccept-Ranges: bytes\r\nContent-Range: bytes {}-{}/{}\r\nContent-Type: audio/mp4\r\nConnection: close\r\n\r\n",
                    body.len(),
                    start,
                    end - 1,
                    bytes.len()
                );
                if socket.write_all(header.as_bytes()).is_err() {
                    continue;
                }
                let first = if range.is_some() {
                    body.len()
                } else {
                    body.len().saturating_sub(tail_bytes)
                };
                if socket.write_all(&body[..first]).is_err() {
                    continue;
                }
                let tail = body[first..].to_vec();
                let delay = if drop_body { 2000 } else { pause_ms };
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_millis(delay));
                    if !drop_body {
                        let _ = socket.write_all(&tail);
                    }
                });
            }
        });
        (url, stop, log, thread)
    }

    // @test RF-18
    #[test]
    fn http_stream_decodes_range_and_headers_without_output() {
        for name in ["tone.mp3", "tone-fragmented.m4a"] {
            let (url, stop, log, server) = serve_fixture(name, false, 350, 8_000);
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();
            let client = reqwest::Client::new();
            let stream = runtime
                .block_on(HttpStream::new(client, url.parse().unwrap()))
                .unwrap();
            let mut reader = runtime
                .block_on(StreamDownload::from_stream(
                    stream,
                    MemoryStorageProvider,
                    Settings::default().prefetch_bytes(1024),
                ))
                .unwrap();
            let near_end = reader.content_length().unwrap() - 2000;
            reader.seek(SeekFrom::Start(near_end)).unwrap();
            let mut sample = [0; 16];
            assert_eq!(reader.read(&mut sample).unwrap(), 16);
            drop(reader);
            drop(runtime);
            let (tx, rx) = unbounded();
            let handle = std::thread::spawn(move || {
                open_stream(
                    url,
                    StreamOptions {
                        headers: vec![("x-test".into(), "miupy".into())],
                        duration_ms: 2000,
                    },
                    1,
                    tx,
                )
            });
            let Cmd::StreamReady(_, mut decoder, _) =
                rx.recv_timeout(Duration::from_secs(5)).unwrap()
            else {
                panic!("stream não abriu: {name}")
            };
            assert!(decoder.next().is_some(), "sem áudio: {name}");
            decoder.try_seek(Duration::from_millis(1800)).unwrap();
            assert!(decoder.next().is_some(), "seek sem áudio: {name}");
            drop(decoder);
            handle.join().unwrap().unwrap();
            stop.store(true, Ordering::Relaxed);
            server.join().unwrap();
            let requests = log.lock().unwrap();
            assert!(
                requests.iter().any(|r| r.contains("x-test: miupy")),
                "cabeçalho ausente: {name}"
            );
            assert!(
                requests
                    .iter()
                    .any(|r| r.to_ascii_lowercase().contains("range: bytes=")),
                "Range ausente: {name}"
            );
        }
    }

    // @test RF-18
    #[test]
    fn http_stream_reports_interrupted_connection() {
        let (url, stop, log, server) = serve_fixture("tone-long-fragmented.m4a", true, 0, 1024);
        let (tx, rx) = unbounded();
        let handle = std::thread::spawn(move || {
            if let Err(e) = open_stream(url, StreamOptions::default(), 1, tx.clone()) {
                let _ = tx.send(Cmd::StreamFailed(1, e));
            }
        });
        let mut got_error = false;
        let mut decoder = None;
        let mut failure = None;
        let deadline = Instant::now() + Duration::from_secs(25);
        while Instant::now() < deadline {
            match rx.recv_timeout(Duration::from_millis(500)) {
                Ok(Cmd::StreamFailed(_, _)) => {
                    got_error = true;
                    break;
                }
                Ok(Cmd::StreamReady(_, source, flag)) => {
                    decoder = Some(source);
                    failure = Some(flag);
                    break;
                }
                Ok(_) => {}
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
                Err(_) => {}
            }
        }
        // A conexão cai antes de o arquivo chegar inteiro. O erro pode vir já na abertura
        // (o decoder precisa do arquivo todo) ou, se abriu, durante a leitura.
        if let Some(source) = decoder.take() {
            std::thread::sleep(Duration::from_secs(3));
            let _ = source.count();
            got_error |= failure.unwrap().lock().unwrap().is_some();
        }
        assert!(got_error, "queda de rede não foi sinalizada");
        assert!(
            log.lock()
                .unwrap()
                .iter()
                .any(|r| r.to_ascii_lowercase().contains("range: bytes="))
        );
        stop.store(true, Ordering::Relaxed);
        server.join().unwrap();
        handle.join().unwrap();
    }

    // @test RF-18
    #[test]
    fn stream_opens_once_the_whole_file_arrives() {
        // O servidor segura o último 1 KiB por 2 s. O decoder busca por bytes e por isso só
        // fica pronto com o arquivo inteiro: a abertura espera a cauda, sem virar erro.
        let (url, stop, _, server) = serve_fixture("tone-fragmented.m4a", false, 2000, 1024);
        let (tx, rx) = unbounded();
        let handle = std::thread::spawn(move || open_stream(url, StreamOptions::default(), 1, tx));
        let result = rx.recv_timeout(Duration::from_secs(5));
        assert!(
            matches!(result, Ok(Cmd::StreamReady(..))),
            "stream não ficou pronto: {:?}",
            result.err()
        );
        drop(result);
        stop.store(true, Ordering::Relaxed);
        server.join().unwrap();
        handle.join().unwrap().unwrap();
    }

    // @test RF-18
    #[test]
    fn load_url_returns_before_network_and_reports_network_error() {
        let (url, stop, _, server) = serve_fixture("tone-fragmented.m4a", false, 2000, 1024);
        let b = RodioBackend::new().unwrap();
        let started = Instant::now();
        b.load_url(
            &url,
            StreamOptions {
                headers: vec![],
                duration_ms: 2000,
            },
        )
        .unwrap();
        assert!(started.elapsed() < Duration::from_millis(300));
        assert_eq!(b.state().status, PlaybackStatus::Loading);
        assert_eq!(b.state().duration_ms, 2000);
        b.stop().unwrap();
        stop.store(true, Ordering::Relaxed);
        server.join().unwrap();

        let rx = b.events();
        b.load_url(
            "http://127.0.0.1:1/unavailable.m4a",
            StreamOptions::default(),
        )
        .unwrap();
        assert!(wait_for(&rx, 5, |e| matches!(e, AudioEvent::Error { .. })));
        assert_eq!(b.state().status, PlaybackStatus::Stopped);
    }

    // @test RF-18 RF-03
    #[test]
    fn http_backend_plays_seeks_and_ends_when_output_exists() {
        for name in ["tone-fragmented.m4a", "tone.mp3"] {
            let (url, stop, _, server) = serve_fixture(name, false, 100, 1024);
            let b = RodioBackend::new().unwrap();
            let rx = b.events();
            b.load_url(
                &url,
                StreamOptions {
                    headers: vec![],
                    duration_ms: 2000,
                },
            )
            .unwrap();
            b.play().unwrap();
            let deadline = Instant::now() + Duration::from_secs(5);
            while b.state().status == PlaybackStatus::Loading && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(10));
            }
            if b.state().status == PlaybackStatus::Stopped {
                let events: Vec<_> = rx.try_iter().collect();
                if events.iter().any(|e| matches!(e, AudioEvent::Error { message } if message.contains("dispositivo de saída"))) {
                    eprintln!("PULADO: streaming com saída real ({name}): sem dispositivo");
                    stop.store(true, Ordering::Relaxed);
                    server.join().unwrap();
                    return;
                }
                panic!("stream parou antes de tocar {name}: {events:?}");
            }
            assert_eq!(b.state().status, PlaybackStatus::Playing, "{name}");
            b.seek(1200).unwrap();
            assert!(
                (1100..=1300).contains(&b.state().position_ms),
                "posição {name}"
            );
            assert!(
                wait_for(&rx, 5, |e| *e == AudioEvent::TrackEnded),
                "sem fim: {name}"
            );
            stop.store(true, Ordering::Relaxed);
            server.join().unwrap();
        }
    }

    // @test RF-18
    #[test]
    fn http_stream_survives_idle_after_download_completes() {
        // A faixa inteira chega em milissegundos; depois disso o download não progride mais.
        // Ficar parado (pausado) por mais tempo que o vigia de rede não pode virar erro.
        let (url, stop, _, server) = serve_fixture("tone-long-fragmented.m4a", false, 0, 0);
        let b = RodioBackend::new().unwrap();
        let rx = b.events();
        b.load_url(
            &url,
            StreamOptions {
                headers: vec![],
                duration_ms: 5000,
            },
        )
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while b.state().status == PlaybackStatus::Loading && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        if b.state().status == PlaybackStatus::Stopped {
            eprintln!("PULADO: streaming ocioso com saída real: sem dispositivo");
            stop.store(true, Ordering::Relaxed);
            server.join().unwrap();
            return;
        }
        assert_eq!(b.state().status, PlaybackStatus::Paused);
        std::thread::sleep(Duration::from_secs(8));
        let events: Vec<_> = rx.try_iter().collect();
        assert!(
            !events.iter().any(|e| matches!(e, AudioEvent::Error { .. })),
            "erro com o download já completo: {events:?}"
        );
        assert_eq!(b.state().status, PlaybackStatus::Paused);
        b.play().unwrap();
        assert!(
            wait_for(&rx, 8, |e| *e == AudioEvent::TrackEnded),
            "não chegou ao fim depois de ficar ocioso"
        );
        stop.store(true, Ordering::Relaxed);
        server.join().unwrap();
    }

    // @test RF-18
    /// Prova contra a rede real. Rode com a URL de áudio resolvida pelo yt-dlp:
    /// `MIUPY_STREAM_URL="$(yt-dlp -g -f 140 <vídeo>)" MIUPY_STREAM_MS=<duração> \
    ///  cargo test -p miupy-audio real_stream -- --ignored --nocapture`
    #[test]
    #[ignore = "rede + URL real em MIUPY_STREAM_URL + dispositivo de saída"]
    fn real_stream_plays_seeks_and_keeps_playing() {
        let url = std::env::var("MIUPY_STREAM_URL").expect("defina MIUPY_STREAM_URL");
        let duration_ms: u64 = std::env::var("MIUPY_STREAM_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let b = RodioBackend::new().unwrap();
        let rx = b.events();
        b.set_muted(true).unwrap();
        let started = Instant::now();
        b.load_url(
            &url,
            StreamOptions {
                headers: vec![],
                duration_ms,
            },
        )
        .unwrap();
        b.play().unwrap();
        let deadline = Instant::now() + Duration::from_secs(20);
        while b.state().status == PlaybackStatus::Loading && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
        }
        let events: Vec<_> = rx.try_iter().collect();
        assert_eq!(
            b.state().status,
            PlaybackStatus::Playing,
            "não começou a tocar: {events:?}"
        );
        println!("primeiro áudio em {} ms", started.elapsed().as_millis());
        // mais que o tempo limite de 5 s do cliente HTTP e do vigia de rede
        std::thread::sleep(Duration::from_secs(12));
        let pos = b.state().position_ms;
        println!("posição depois de 12 s: {pos} ms");

        assert!((10_000..=13_500).contains(&pos), "posição {pos}");
        let target = if duration_ms > 90_000 { 60_000 } else { 2_000 };
        let seek_started = Instant::now();
        b.seek(target).unwrap();
        std::thread::sleep(Duration::from_secs(4));
        let after = b.state().position_ms;
        println!(
            "seek para {target} ms levou {} ms; posição 4 s depois: {after} ms",
            seek_started.elapsed().as_millis().saturating_sub(4000)
        );
        let events: Vec<_> = rx.try_iter().collect();
        assert!(
            !events.iter().any(|e| matches!(e, AudioEvent::Error { .. })),
            "erro durante o streaming real: {:?}",
            events
                .iter()
                .filter(|e| matches!(e, AudioEvent::Error { .. }))
                .collect::<Vec<_>>()
        );
        assert_eq!(b.state().status, PlaybackStatus::Playing);
        assert!(
            (target + 3_000..=target + 5_500).contains(&after),
            "posição depois do seek: {after}"
        );
    }

    // @test RF-18
    #[test]
    fn http_backend_reports_midstream_drop_and_recovers() {
        let (url, stop, _, server) = serve_fixture("tone-long-fragmented.m4a", true, 0, 1024);
        let b = RodioBackend::new().unwrap();
        let rx = b.events();
        b.load_url(
            &url,
            StreamOptions {
                headers: vec![],
                duration_ms: 5000,
            },
        )
        .unwrap();
        match b.play() {
            Ok(()) => {}
            // Sem placa de som (ex.: runner do CI), como nos outros testes com saída real.
            Err(AudioError::NoOutputDevice(why)) => {
                eprintln!("PULADO: queda no meio do stream com saída real ({why})");
                stop.store(true, Ordering::Relaxed);
                server.join().unwrap();
                return;
            }
            Err(e) => panic!("play falhou: {e}"),
        }
        let ready_deadline = Instant::now() + Duration::from_secs(1);
        while b.state().status == PlaybackStatus::Loading && Instant::now() < ready_deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        let was_playing = b.state().status == PlaybackStatus::Playing;
        if was_playing {
            assert_eq!(b.state().duration_ms, 5000);
        }
        let deadline = Instant::now() + Duration::from_secs(25);
        let mut error = None;
        while Instant::now() < deadline {
            match rx.recv_timeout(Duration::from_millis(200)) {
                Ok(AudioEvent::Error { message }) => {
                    error = Some(message);
                    break;
                }
                Ok(AudioEvent::TrackEnded) => panic!("queda de rede confundida com fim natural"),
                Ok(_) | Err(_) => {}
            }
        }
        if error
            .as_ref()
            .is_some_and(|e| e.contains("dispositivo de saída"))
        {
            eprintln!("PULADO: queda durante streaming com saída real: sem dispositivo");
        } else {
            let _ = was_playing;
            assert!(error.is_some(), "queda de rede não emitiu Error");
            assert_eq!(b.state().status, PlaybackStatus::Stopped);
            match b.load(&fixture("tone.m4a")) {
                Ok(()) => assert_eq!(b.state().status, PlaybackStatus::Paused),
                Err(AudioError::NoOutputDevice(why)) => {
                    eprintln!("PULADO: recarga depois da queda com saída real ({why})")
                }
                Err(e) => panic!("recarga falhou: {e}"),
            }
        }
        stop.store(true, Ordering::Relaxed);
        server.join().unwrap();
    }

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
