//! Player: junta o motor de áudio, a fila, o banco e o registro de atividade.
//!
//! O motor (`AudioBackend`) toca um arquivo por vez; aqui ficam a fila, o avanço automático ao
//! fim da faixa, a emissão de `player://state` / `player://progress` e o registro de cada
//! reprodução (início, tempo realmente ouvido, desfecho).
//!
//! Concorrência: todo o estado mutável fica em `Inner`, atrás de um `Mutex`. Os comandos e a
//! thread que consome `backend.events()` (UM único consumidor) travam `Inner` e, dentro dele,
//! o banco, sempre nessa ordem. Nada aqui é `async`.

use std::path::PathBuf;
use std::sync::{
    Arc, Mutex, MutexGuard,
    atomic::{AtomicU64, Ordering},
};

use miupy_audio::{AudioBackend, AudioEvent, PlaybackStatus, StreamOptions};
use miupy_core::{PlayEnd, SettingsPatch, ShuffleMode, Source, Track};
use miupy_media::{MediaResolver, StreamAudio};

use crate::db::Db;
use crate::dto::{PlayerState, PlayerStatus, Progress};
use crate::error::{AppError, AppResult};
use crate::events::Events;
use crate::listening::ListenTracker;
use crate::paths::Prefs;
use crate::queue::PlayQueue;

/// Quanto tempo ouvido acumula antes de ir para o banco (o resto sai na pausa/troca/parada).
const FLUSH_LISTENED_MS: u64 = 2_000;
/// "Anterior" depois disso reinicia a faixa em vez de voltar na fila.
const RESTART_THRESHOLD_MS: u64 = 3_000;

struct Inner {
    queue: PlayQueue,
    track: Option<Track>,
    /// Evento de reprodução aberto (RF-13) da faixa atual.
    event_id: Option<String>,
    tracker: ListenTracker,
    /// Último status informado pelo motor (espelho, atualizado pela thread de eventos).
    status: PlaybackStatus,
    streaming: bool,
    stream_expires_at: Option<u64>,
    stream_retried: bool,
    resolving: bool,
    pause_requested: bool,
    shuffle: ShuffleMode,
}

pub struct Player {
    db: Db,
    backend: Arc<dyn AudioBackend>,
    resolver: Arc<dyn MediaResolver>,
    events: Arc<dyn Events>,
    prefs_file: PathBuf,
    inner: Mutex<Inner>,
    generation: AtomicU64,
}

impl Player {
    pub fn new(
        db: Db,
        backend: Arc<dyn AudioBackend>,
        resolver: Arc<dyn MediaResolver>,
        events: Arc<dyn Events>,
        prefs_file: PathBuf,
    ) -> Arc<Self> {
        Arc::new(Player {
            db,
            backend,
            resolver,
            events,
            prefs_file,
            inner: Mutex::new(Inner {
                queue: PlayQueue::default(),
                track: None,
                event_id: None,
                tracker: ListenTracker::default(),
                status: PlaybackStatus::Stopped,
                streaming: false,
                stream_expires_at: None,
                stream_retried: false,
                resolving: false,
                pause_requested: false,
                shuffle: ShuffleMode::Off,
            }),
            generation: AtomicU64::new(0),
        })
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    // @req RF-02
    /// Aplica ao motor o volume salvo em `settings` e o mudo salvo em `prefs.json`.
    pub fn restore_prefs(&self) {
        match self.db.with(|db| db.get_settings()) {
            Ok(s) => {
                if let Err(e) = self.backend.set_volume(s.volume as f32) {
                    log::warn!("não consegui aplicar o volume salvo: {e}");
                }
            }
            Err(e) => log::warn!("não consegui ler as configurações: {e}"),
        }
        let prefs = Prefs::load(&self.prefs_file);
        if let Err(e) = self.backend.set_muted(prefs.muted) {
            log::warn!("não consegui aplicar o mudo salvo: {e}");
        }
        if let Ok(mode) = self.db.with(|db| db.get_shuffle_mode()) {
            self.lock().shuffle = mode;
        }
    }

    /// Thread que consome `backend.events()` (único consumidor) e repassa ao frontend.
    pub fn spawn_event_loop(self: &Arc<Self>) {
        let rx = self.backend.events();
        let this = Arc::clone(self);
        let spawned = std::thread::Builder::new()
            .name("miupy-player-events".into())
            .spawn(move || {
                while let Ok(ev) = rx.recv() {
                    this.handle_event(ev);
                }
                log::info!("canal de eventos do áudio fechado");
            });
        if let Err(e) = spawned {
            log::error!("não consegui iniciar a thread de eventos do áudio: {e}");
        }
    }

    /// Processa, sem bloquear, os eventos já enfileirados (usado nos testes, que não têm thread).
    #[cfg(test)]
    pub fn process_pending_events(&self) {
        let rx = self.backend.events();
        while let Ok(ev) = rx.try_recv() {
            self.handle_event(ev);
        }
    }

    // ------------------------------------------------------------------ estado

    pub fn state(&self) -> PlayerState {
        let inner = self.lock();
        self.snapshot(&inner)
    }

    // @req RF-03
    pub fn queue_tracks(&self) -> AppResult<Vec<Track>> {
        let ids = self.state().queue;
        Ok(self.db.with(|db| db.get_tracks_by_ids(&ids))?)
    }

    fn snapshot(&self, inner: &Inner) -> PlayerState {
        let st = self.backend.state();
        // Relê a faixa: favorito e capa podem ter mudado desde que ela começou a tocar.
        let track = inner.track.as_ref().map(|t| {
            self.db
                .with(|db| db.get_track(&t.id))
                .unwrap_or_else(|_| t.clone())
        });
        let status = if inner.resolving {
            PlayerStatus::Loading
        } else if track.is_some() {
            PlayerStatus::from(st.status)
        } else {
            PlayerStatus::Stopped
        };
        let duration_ms = if st.duration_ms > 0 {
            st.duration_ms
        } else {
            track.as_ref().map_or(0, |t| t.duration_ms.max(0) as u64)
        };
        PlayerState {
            status,
            streaming: inner.streaming && track.is_some() && status != PlayerStatus::Stopped,
            shuffle: inner.shuffle,
            track,
            position_ms: st.position_ms,
            duration_ms,
            volume: st.volume,
            muted: st.muted,
            queue: inner.queue.ids().to_vec(),
            queue_index: inner.queue.index_i64(),
        }
    }

    fn emit(&self, inner: &Inner) {
        self.events.player_state(&self.snapshot(inner));
    }

    // ------------------------------------------------------- registro (RF-13)

    /// Fecha o evento de reprodução aberto, gravando antes o tempo ouvido pendente.
    fn close_event(&self, inner: &mut Inner, end: PlayEnd) {
        let pending = inner.tracker.take_pending();
        inner.tracker.reset();
        let Some(id) = inner.event_id.take() else {
            return;
        };
        let res = self.db.with(|db| {
            if pending > 0 {
                db.add_listened(&id, pending as i64)?;
            }
            db.finish_play(&id, end).map(|_| ())
        });
        if let Err(e) = res {
            log::debug!("não consegui fechar o evento de reprodução {id}: {e}");
        }
    }

    /// Grava o tempo ouvido acumulado quando passa do limite (ou sempre, com `force`).
    fn flush_listened(&self, inner: &mut Inner, force: bool) {
        if !force && inner.tracker.pending_ms() < FLUSH_LISTENED_MS {
            return;
        }
        let Some(id) = inner.event_id.clone() else {
            return;
        };
        let pending = inner.tracker.take_pending();
        if pending == 0 {
            return;
        }
        if let Err(e) = self.db.with(|db| db.add_listened(&id, pending as i64)) {
            log::debug!("não consegui gravar o tempo ouvido: {e}");
        }
    }

    // ----------------------------------------------------------- reprodução

    /// Carrega e toca `track`, fechando o evento da faixa anterior com `end_prev`.
    fn start_track(&self, inner: &mut Inner, track: Track, end_prev: PlayEnd) -> AppResult<()> {
        let stream = if track.file_path.is_none() && track.source == Source::Youtube {
            Some(self.resolve_track_stream(&track)?)
        } else {
            None
        };
        let path = if stream.is_none() {
            Some(playable_path(&track)?)
        } else {
            None
        };
        self.close_event(inner, end_prev);
        // @req RF-28
        let gain = if stream.is_none() {
            let mode = self.db.with(|db| db.get_settings())?.normalization;
            let peak = self.db.with(|db| db.get_true_peak_db(&track.id))?;
            mode.gain_db(track.loudness_lufs, peak)
        } else {
            0.0
        };
        self.backend.set_normalization_gain_db(gain)?;
        let loaded = if let Some(stream) = &stream {
            self.load_stream(stream, &track, true)
        } else {
            self.backend
                .load(path.as_deref().expect("caminho validado"))
                .and_then(|_| self.backend.play())
                .map_err(AppError::from)
        };
        if let Err(e) = loaded {
            let _ = self.backend.stop();
            inner.track = None;
            inner.status = PlaybackStatus::Stopped;
            return Err(e);
        }
        inner.event_id = match self.db.with(|db| db.start_play(&track.id, None)) {
            Ok(id) => Some(id),
            Err(e) => {
                log::warn!("não consegui abrir o evento de reprodução: {e}");
                None
            }
        };
        inner.tracker.reset();
        inner.status = self.backend.state().status;
        inner.streaming = stream.is_some();
        inner.stream_expires_at = stream.and_then(|s| s.expires_at);
        inner.stream_retried = false;
        inner.track = Some(track);
        Ok(())
    }

    // @req RF-18
    fn resolve_track_stream(&self, track: &Track) -> AppResult<StreamAudio> {
        let id = track
            .source_id
            .clone()
            .ok_or_else(|| AppError::unavailable("Faixa sem origem para streaming."))?;
        let resolver = self.resolver.clone();
        std::thread::spawn(move || tauri::async_runtime::block_on(resolver.resolve_stream(&id)))
            .join()
            .map_err(|_| AppError::internal("Resolução do streaming interrompida."))?
            .map_err(AppError::from)
    }

    // @req RF-01 RF-18
    fn load_stream(&self, stream: &StreamAudio, track: &Track, autoplay: bool) -> AppResult<()> {
        let options = StreamOptions {
            headers: stream
                .http_headers
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            duration_ms: stream
                .duration_ms
                .unwrap_or(track.duration_ms.max(0) as u64),
        };
        self.backend.load_url(&stream.url, options)?;
        if autoplay {
            self.backend.play()?;
        }
        Ok(())
    }

    /// Toca a faixa que está na posição `idx` da fila e a torna a atual.
    fn start_index(&self, inner: &mut Inner, idx: usize, end_prev: PlayEnd) -> AppResult<()> {
        let id = inner
            .queue
            .id_at(idx)
            .ok_or_else(|| AppError::not_found("posição inexistente na fila"))?
            .to_owned();
        let track = self.db.with(|db| db.get_track(&id))?;
        self.start_track(inner, track, end_prev)?;
        inner.queue.set_index(idx);
        Ok(())
    }

    /// Anda pela fila a partir de `from` na direção dada, pulando faixas que não dá para tocar.
    /// `Ok(false)` se não havia nenhuma faixa tocável adiante.
    fn walk(
        &self,
        inner: &mut Inner,
        from: usize,
        forward: bool,
        end_prev: PlayEnd,
    ) -> AppResult<bool> {
        let mut cur = from;
        loop {
            let step = if forward {
                inner.queue.next_after(cur)
            } else {
                inner.queue.prev_before(cur)
            };
            let Some(idx) = step else {
                return Ok(false);
            };
            match self.start_index(inner, idx, end_prev) {
                Ok(()) => return Ok(true),
                Err(e) if e.is_track_specific() => {
                    log::warn!("pulando faixa da fila: {e}");
                    cur = idx;
                }
                Err(e) => return Err(e),
            }
        }
    }

    // @req RF-01 RF-03
    /// Toca `track_id`. Com `queue`, ela vira a fila (a faixa entra no começo se faltar);
    /// sem ela, usa a fila atual se a faixa estiver nela, ou uma fila só com a faixa.
    pub fn play(
        self: &Arc<Self>,
        track_id: &str,
        queue: Option<Vec<String>>,
    ) -> AppResult<PlayerState> {
        let track = self.db.with(|db| db.get_track(track_id))?;
        let mut inner = self.lock();
        let new_queue = queue.is_some();
        let mut q = inner.queue.clone();
        match queue {
            Some(ids) => q.replace(ids, &track.id),
            None => {
                if !q.select(&track.id) {
                    q.replace(Vec::new(), &track.id);
                }
            }
        }
        if new_queue && inner.shuffle != ShuffleMode::Off {
            // Fila nova com aleatório: a faixa pedida vai primeiro e todas as outras são sorteadas.
            self.shuffle_after_current(&mut q, inner.shuffle, true)?;
        }
        inner.queue = q;
        let is_stream = track.file_path.is_none() && track.source == Source::Youtube;
        if is_stream {
            let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
            self.close_event(&mut inner, PlayEnd::Skipped);
            let _ = self.backend.stop();
            inner.track = Some(track.clone());
            inner.status = PlaybackStatus::Loading;
            inner.streaming = true;
            inner.resolving = true;
            inner.pause_requested = false;
            inner.stream_expires_at = None;
            inner.stream_retried = false;
            inner.tracker.reset();
            self.emit(&inner);
            let this = Arc::clone(self);
            std::thread::spawn(move || {
                let resolved = this.resolve_track_stream(&track);
                let mut inner = this.lock();
                if this.generation.load(Ordering::SeqCst) != generation {
                    return;
                }
                inner.resolving = false;
                match resolved.and_then(|stream| {
                    this.load_stream(&stream, &track, !inner.pause_requested)?;
                    inner.event_id = this.db.with(|db| db.start_play(&track.id, None)).ok();
                    inner.tracker.reset();
                    inner.status = this.backend.state().status;
                    inner.streaming = true;
                    inner.stream_expires_at = stream.expires_at;
                    inner.stream_retried = false;
                    Ok(())
                }) {
                    Ok(()) => this.emit(&inner),
                    Err(error) => {
                        let _ = this.backend.stop();
                        inner.track = None;
                        inner.streaming = false;
                        inner.status = PlaybackStatus::Stopped;
                        this.events
                            .player_error(&track.id, error.code(), &error.to_string());
                        this.emit(&inner);
                    }
                }
            });
            return Ok(self.snapshot(&inner));
        }
        self.generation.fetch_add(1, Ordering::SeqCst);
        inner.resolving = false;
        inner.pause_requested = false;
        self.start_track(&mut inner, track, PlayEnd::Skipped)?;
        self.emit(&inner);
        Ok(self.snapshot(&inner))
    }

    // @req RF-24
    /// Sorteia o que vem depois da faixa atual. Com `whole_queue`, ela vai para o começo e
    /// todas as outras faixas entram no sorteio (fila nova); sem ele, o que já passou fica antes.
    fn shuffle_after_current(
        &self,
        queue: &mut PlayQueue,
        mode: ShuffleMode,
        whole_queue: bool,
    ) -> AppResult<()> {
        let ids = queue.ids().to_vec();
        let Some(index) = queue.index() else {
            return Ok(());
        };
        let current = &ids[index];
        let (mut out, rest): (Vec<String>, Vec<String>) = if whole_queue {
            let mut others = ids.clone();
            others.remove(index);
            (vec![current.clone()], others)
        } else {
            (ids[..=index].to_vec(), ids[index + 1..].to_vec())
        };
        let tail = rest.as_slice();
        let seed = now_seconds();
        let ordered = match mode {
            ShuffleMode::Off => queue.original_order_of(&out, tail),
            ShuffleMode::Random => miupy_core::random_order(tail, seed),
            ShuffleMode::Smart => {
                let smart = self.db.with(|db| db.smart_tracks(&ids))?;
                let by_id = smart
                    .into_iter()
                    .map(|t| (t.track_id.clone(), t))
                    .collect::<std::collections::HashMap<_, _>>();
                let current_track = by_id.get(current);
                if let Some(cur) = current_track {
                    let rem = tail
                        .iter()
                        .filter_map(|id| by_id.get(id).cloned())
                        .collect::<Vec<_>>();
                    if rem.len() == tail.len() {
                        miupy_core::smart_order(cur, &rem, seed)
                    } else {
                        miupy_core::random_order(tail, seed)
                    }
                } else {
                    miupy_core::random_order(tail, seed)
                }
            }
        };
        out.extend(ordered);
        queue.set_ids(out, if whole_queue { 0 } else { index });
        Ok(())
    }

    // @req RF-24
    pub fn set_shuffle(&self, mode: ShuffleMode) -> AppResult<PlayerState> {
        let mut inner = self.lock();
        self.db.with(|db| db.set_shuffle_mode(mode))?;
        inner.shuffle = mode;
        if inner.queue.index().is_some() {
            let mut queue = inner.queue.clone();
            self.shuffle_after_current(&mut queue, mode, false)?;
            inner.queue = queue;
        }
        self.emit(&inner);
        Ok(self.snapshot(&inner))
    }

    // @req RF-01 RF-18
    pub fn pause(&self) -> AppResult<PlayerState> {
        let mut inner = self.lock();
        if inner.resolving {
            inner.pause_requested = true;
            self.emit(&inner);
        } else {
            self.backend.pause()?;
        }
        Ok(self.snapshot(&inner))
    }

    // @req RF-01 RF-18
    /// Retoma. Se a faixa já terminou ou foi parada, recomeça a faixa atual da fila.
    pub fn resume(&self) -> AppResult<PlayerState> {
        let mut inner = self.lock();
        if inner.resolving {
            inner.pause_requested = false;
            self.emit(&inner);
            return Ok(self.snapshot(&inner));
        }
        if inner.streaming
            && inner
                .stream_expires_at
                .is_some_and(|expires| now_seconds() >= expires)
        {
            let track = inner
                .track
                .clone()
                .ok_or_else(|| AppError::unavailable("Não há faixa para retomar."))?;
            let position = self.backend.state().position_ms;
            let stream = self.resolve_track_stream(&track)?;
            self.load_stream(&stream, &track, true)?;
            if position > 0 {
                self.backend.seek(position)?;
            }
            inner.stream_expires_at = stream.expires_at;
            inner.stream_retried = false;
            inner.status = self.backend.state().status;
            self.emit(&inner);
            return Ok(self.snapshot(&inner));
        }
        if self.backend.state().status == PlaybackStatus::Stopped {
            let idx = match (inner.queue.index(), inner.track.is_some()) {
                (Some(i), true) => i,
                _ => return Err(AppError::unavailable("Não há nada para tocar.")),
            };
            self.start_index(&mut inner, idx, PlayEnd::Stopped)?;
            self.emit(&inner);
        } else {
            self.backend.resume()?;
        }
        Ok(self.snapshot(&inner))
    }

    // @req RF-01
    pub fn stop(&self) -> AppResult<PlayerState> {
        self.generation.fetch_add(1, Ordering::SeqCst);
        let mut inner = self.lock();
        inner.resolving = false;
        inner.pause_requested = false;
        self.close_event(&mut inner, PlayEnd::Stopped);
        self.backend.stop()?;
        inner.status = PlaybackStatus::Stopped;
        self.emit(&inner);
        Ok(self.snapshot(&inner))
    }

    // @req RF-03
    pub fn seek(&self, position_ms: u64) -> AppResult<PlayerState> {
        let mut inner = self.lock();
        self.backend.seek(position_ms)?;
        let now = self.backend.state().position_ms;
        inner.tracker.seek(now);
        Ok(self.snapshot(&inner))
    }

    // @req RF-03
    /// Próxima da fila. No fim da fila não faz nada.
    pub fn next(&self) -> AppResult<PlayerState> {
        let mut inner = self.lock();
        if let Some(cur) = inner.queue.index()
            && self.walk(&mut inner, cur, true, PlayEnd::Skipped)?
        {
            self.generation.fetch_add(1, Ordering::SeqCst);
            inner.resolving = false;
            inner.pause_requested = false;
            self.emit(&inner);
        }
        Ok(self.snapshot(&inner))
    }

    // @req RF-03
    /// Se já tocou mais de 3 s (ou é a primeira da fila), reinicia a faixa; senão volta uma.
    pub fn previous(&self) -> AppResult<PlayerState> {
        let mut inner = self.lock();
        let Some(cur) = inner.queue.index() else {
            return Ok(self.snapshot(&inner));
        };
        let st = self.backend.state();
        if st.position_ms > RESTART_THRESHOLD_MS || cur == 0 {
            if st.status == PlaybackStatus::Stopped {
                if inner.track.is_some() {
                    self.start_index(&mut inner, cur, PlayEnd::Stopped)?;
                    self.generation.fetch_add(1, Ordering::SeqCst);
                    inner.resolving = false;
                    inner.pause_requested = false;
                    self.emit(&inner);
                }
            } else {
                self.backend.seek(0)?;
                inner.tracker.seek(0);
            }
        } else if self.walk(&mut inner, cur, false, PlayEnd::Skipped)? {
            self.generation.fetch_add(1, Ordering::SeqCst);
            inner.resolving = false;
            inner.pause_requested = false;
            self.emit(&inner);
        }
        Ok(self.snapshot(&inner))
    }

    // @req RF-02
    // @req RF-28
    pub fn refresh_normalization(&self) -> AppResult<()> {
        let current = self.lock().track.clone();
        let gain = if let Some(track) = current.filter(|t| t.file_path.is_some()) {
            let mode = self.db.with(|db| db.get_settings())?.normalization;
            let peak = self.db.with(|db| db.get_true_peak_db(&track.id))?;
            mode.gain_db(track.loudness_lufs, peak)
        } else {
            0.0
        };
        self.backend.set_normalization_gain_db(gain)?;
        Ok(())
    }

    /// Define o volume (0.0 a 1.0), aplica ao motor e persiste em `settings`.
    pub fn set_volume(&self, volume: f64) -> AppResult<PlayerState> {
        if !volume.is_finite() {
            return Err(AppError::invalid("volume inválido"));
        }
        let volume = (volume.clamp(0.0, 1.0) * 1000.0).round() / 1000.0;
        let inner = self.lock();
        self.backend.set_volume(volume as f32)?;
        self.db.with(|db| {
            db.update_settings(&SettingsPatch {
                volume: Some(volume),
                ..Default::default()
            })
        })?;
        Ok(self.snapshot(&inner))
    }

    // @req RF-02
    pub fn set_muted(&self, muted: bool) -> AppResult<PlayerState> {
        let inner = self.lock();
        self.backend.set_muted(muted)?;
        if let Err(e) = (Prefs { muted }).save(&self.prefs_file) {
            log::warn!("não consegui salvar o mudo: {e}");
        }
        Ok(self.snapshot(&inner))
    }

    /// A faixa foi apagada da biblioteca: sai da fila e, se estava tocando, para.
    pub fn remove_track(&self, track_id: &str) {
        let mut inner = self.lock();
        let was_current = inner.track.as_ref().is_some_and(|t| t.id == track_id);
        inner.queue.remove(track_id);
        if was_current {
            self.close_event(&mut inner, PlayEnd::Stopped);
            let _ = self.backend.stop();
            inner.track = None;
            inner.status = PlaybackStatus::Stopped;
        }
        self.emit(&inner);
    }

    /// Ao fechar o app: fecha o evento aberto como `Stopped` e silencia o motor.
    pub fn shutdown(&self) {
        let mut inner = self.lock();
        self.close_event(&mut inner, PlayEnd::Stopped);
        let _ = self.backend.stop();
    }

    // ------------------------------------------------------ eventos do motor

    pub fn handle_event(&self, ev: AudioEvent) {
        match ev {
            AudioEvent::StateChanged(st) => {
                let mut inner = self.lock();
                let was_playing = inner.status == PlaybackStatus::Playing;
                inner.status = st.status;
                if st.status == PlaybackStatus::Playing {
                    inner.tracker.observe(st.position_ms);
                    self.flush_listened(&mut inner, false);
                } else {
                    // O trecho entre o último progresso e a pausa/fim também foi ouvido.
                    if was_playing && inner.event_id.is_some() {
                        inner.tracker.observe(st.position_ms);
                    }
                    inner.tracker.pause();
                    self.flush_listened(&mut inner, true);
                }
                self.emit(&inner);
            }
            AudioEvent::Progress {
                position_ms,
                duration_ms,
            } => {
                {
                    let mut inner = self.lock();
                    if inner.status == PlaybackStatus::Playing && inner.event_id.is_some() {
                        inner.tracker.observe(position_ms);
                        self.flush_listened(&mut inner, false);
                    }
                }
                self.events.player_progress(Progress {
                    position_ms,
                    duration_ms,
                });
            }
            AudioEvent::TrackEnded => self.on_track_ended(),
            AudioEvent::Error { message } => {
                log::warn!("erro do motor de áudio: {message}");
                let mut inner = self.lock();
                if inner.track.is_none() {
                    return;
                }
                if inner.streaming && !inner.stream_retried {
                    inner.stream_retried = true;
                    if let Some(track) = inner.track.clone()
                        && let Ok(stream) = self.resolve_track_stream(&track)
                        && self.load_stream(&stream, &track, true).is_ok()
                    {
                        inner.stream_expires_at = stream.expires_at;
                        inner.status = self.backend.state().status;
                        self.emit(&inner);
                        return;
                    }
                }
                self.close_event(&mut inner, PlayEnd::Stopped);
                let _ = self.backend.stop();
                inner.status = PlaybackStatus::Stopped;
                inner.track = None;
                self.emit(&inner);
                if let Some(cur) = inner.queue.index() {
                    match self.walk(&mut inner, cur, true, PlayEnd::Stopped) {
                        Ok(_) => self.emit(&inner),
                        Err(e) => log::warn!("falha ao avançar após erro de áudio: {e}"),
                    }
                }
            }
        }
    }

    // @req RF-03 RF-13
    /// Fim natural da faixa: fecha o evento como concluído e avança a fila.
    fn on_track_ended(&self) {
        let mut inner = self.lock();
        // Um `TrackEnded` atrasado, de uma faixa já substituída por `load`, não avança nada.
        if inner.track.is_none() || self.backend.state().status != PlaybackStatus::Stopped {
            return;
        }
        self.close_event(&mut inner, PlayEnd::Completed);
        inner.status = PlaybackStatus::Stopped;
        if let Some(cur) = inner.queue.index() {
            match self.walk(&mut inner, cur, true, PlayEnd::Completed) {
                Ok(_) => {}
                Err(e) => log::warn!("não consegui avançar a fila: {e}"),
            }
        }
        self.emit(&inner);
    }

    #[cfg(test)]
    pub fn current_event_id(&self) -> Option<String> {
        self.lock().event_id.clone()
    }
}

fn now_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Caminho do arquivo tocável da faixa, ou `unavailable` (RF-01).
fn playable_path(track: &Track) -> AppResult<PathBuf> {
    let Some(fp) = track.file_path.as_deref() else {
        return Err(AppError::unavailable(
            "Esta faixa ainda não foi baixada. Baixe-a para tocar.",
        ));
    };
    let path = PathBuf::from(fp);
    if !path.is_file() {
        return Err(AppError::unavailable(format!(
            "O arquivo da faixa não foi encontrado: {fp}"
        )));
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::testing::RecordingEvents;
    use miupy_audio::NullBackend;
    use miupy_core::{Database, DownloadedFile, NewTrack};

    struct Fx {
        player: Arc<Player>,
        backend: Arc<NullBackend>,
        resolver: Arc<crate::downloads::testing::FakeResolver>,
        events: Arc<RecordingEvents>,
        db: Db,
        dir: tempfile::TempDir,
        ids: Vec<String>,
    }

    /// `n` faixas baixadas (arquivos vazios; o `NullBackend` não decodifica nada).
    fn fx(n: usize) -> Fx {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::new(Database::open_in_memory().unwrap());
        let mut ids = Vec::new();
        for i in 0..n {
            let t = db
                .with(|db| {
                    db.upsert_track(&NewTrack::youtube(
                        format!("vid{i}"),
                        format!("Faixa {i}"),
                        Some("Artista".into()),
                        180_000,
                        None,
                    ))
                })
                .unwrap()
                .track;
            let file = dir.path().join(format!("{}.m4a", t.id));
            std::fs::write(&file, b"x").unwrap();
            db.with(|db| {
                db.mark_downloaded(
                    &t.id,
                    &DownloadedFile {
                        file_path: file.to_string_lossy().into_owned(),
                        codec: Some("aac".into()),
                        bitrate: Some(128),
                        cover_path: None,
                        duration_ms: Some(180_000),
                    },
                )
            })
            .unwrap();
            ids.push(t.id);
        }
        let backend = Arc::new(NullBackend::new());
        let resolver = Arc::new(crate::downloads::testing::FakeResolver::default());
        let events = Arc::new(RecordingEvents::default());
        let player = Player::new(
            db.clone(),
            backend.clone(),
            resolver.clone(),
            events.clone(),
            dir.path().join("prefs.json"),
        );
        Fx {
            player,
            backend,
            resolver,
            events,
            db,
            dir,
            ids,
        }
    }

    impl Fx {
        fn event(&self, id: &str) -> miupy_core::PlayEvent {
            self.db.with(|db| db.get_play_event(id)).unwrap()
        }
        /// Avança o relógio simulado em passos de 500 ms e entrega os eventos.
        fn tick(&self, ms: u64) {
            let mut left = ms;
            while left > 0 {
                let step = left.min(500);
                self.backend.advance(step);
                self.player.process_pending_events();
                left -= step;
            }
        }
    }

    // @test RF-18
    /// Prova real, de ponta a ponta na casca: yt-dlp do sistema resolve a URL e o backend rodio
    /// toca (mudo). `cargo test -p miupy streaming_real -- --ignored --nocapture`
    #[test]
    #[ignore = "rede + yt-dlp instalado + dispositivo de saída"]
    fn streaming_real_toca_avanca_e_registra_atividade() {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::new(Database::open_in_memory().unwrap());
        let track = db
            .with(|db| {
                db.upsert_track(&NewTrack::youtube(
                    "dQw4w9WgXcQ",
                    "Never Gonna Give You Up",
                    Some("Rick Astley".into()),
                    213_000,
                    None,
                ))
            })
            .unwrap()
            .track;
        let backend: Arc<dyn AudioBackend> = Arc::new(miupy_audio::RodioBackend::new().unwrap());
        let player = Player::new(
            db.clone(),
            backend.clone(),
            Arc::new(miupy_media::YtDlpResolver::new()),
            Arc::new(RecordingEvents::default()),
            dir.path().join("prefs.json"),
        );
        player.spawn_event_loop();
        player.set_muted(true).unwrap();
        let started = std::time::Instant::now();
        let state = player.play(&track.id, None).unwrap();
        println!(
            "play devolveu em {} ms com status {:?}, streaming={}",
            started.elapsed().as_millis(),
            state.status,
            state.streaming
        );
        assert!(state.streaming);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        while player.state().status != PlayerStatus::Playing && std::time::Instant::now() < deadline
        {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        println!(
            "tocando {} ms depois do clique",
            started.elapsed().as_millis()
        );
        assert_eq!(player.state().status, PlayerStatus::Playing);
        std::thread::sleep(std::time::Duration::from_secs(8));
        let st = player.state();
        println!("posição depois de 8 s: {} ms", st.position_ms);
        assert!(st.streaming && st.status == PlayerStatus::Playing);
        assert!(
            (6_500..=9_500).contains(&st.position_ms),
            "{}",
            st.position_ms
        );
        player.seek(60_000).unwrap();
        std::thread::sleep(std::time::Duration::from_secs(3));
        let st = player.state();
        println!(
            "posição 3 s depois do seek para 60 s: {} ms",
            st.position_ms
        );
        assert!(
            (62_000..=64_500).contains(&st.position_ms),
            "{}",
            st.position_ms
        );
        let event_id = player
            .current_event_id()
            .expect("evento de reprodução aberto");
        player.stop().unwrap();
        let event = db.with(|db| db.get_play_event(&event_id)).unwrap();
        println!(
            "evento: ouvido {} ms, encerrado={}",
            event.listened_ms,
            event.ended_at.is_some()
        );
        assert!(event.ended_at.is_some());
        assert!(event.listened_ms >= 9_000, "{}", event.listened_ms);
    }

    // @test RF-18
    #[test]
    fn youtube_sem_arquivo_toca_em_streaming() {
        let f = fx(1);
        let t =
            f.db.with(|db| {
                db.upsert_track(&NewTrack::youtube(
                    "semarq",
                    "Sem arquivo",
                    None,
                    1000,
                    None,
                ))
            })
            .unwrap()
            .track;
        let state = f.player.play(&t.id, None).unwrap();
        assert!(state.streaming);
        assert_eq!(state.track.unwrap().id, t.id);
    }

    // @test RF-01 RF-18
    #[test]
    fn pausa_durante_resolucao_carrega_sem_tocar_e_permite_retomar() {
        let f = fx(0);
        let track =
            f.db.with(|db| db.upsert_track(&NewTrack::youtube("pausa", "Pausa", None, 1000, None)))
                .unwrap()
                .track;
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        *f.resolver.stream_gate.lock().unwrap() = Some((started_tx, release_rx));

        let loading = f.player.play(&track.id, None).unwrap();
        assert_eq!(loading.status, PlayerStatus::Loading);
        started_rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        assert_eq!(f.player.pause().unwrap().status, PlayerStatus::Loading);
        release_tx.send(()).unwrap();

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while f.player.state().status == PlayerStatus::Loading
            && std::time::Instant::now() < deadline
        {
            std::thread::yield_now();
        }
        let paused = f.player.state();
        assert_eq!(paused.status, PlayerStatus::Paused);
        assert_eq!(paused.position_ms, 0);
        assert!(paused.streaming);
        assert_eq!(paused.track.as_ref().unwrap().id, track.id);
        assert_eq!(
            f.events.states.lock().unwrap().last().unwrap().status,
            PlayerStatus::Paused
        );
        f.backend.advance(1000);
        f.player.process_pending_events();
        assert_eq!(f.player.state().position_ms, 0);
        assert!(
            f.events
                .states
                .lock()
                .unwrap()
                .iter()
                .all(|state| state.status != PlayerStatus::Playing)
        );
        assert_eq!(f.player.resume().unwrap().status, PlayerStatus::Playing);
    }

    // @test RF-01 RF-18
    #[test]
    fn retomar_durante_resolucao_cancela_pausa() {
        let f = fx(0);
        let track = f
            .db
            .with(|db| db.upsert_track(&NewTrack::youtube("retoma", "Retoma", None, 1000, None)))
            .unwrap()
            .track;
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        *f.resolver.stream_gate.lock().unwrap() = Some((started_tx, release_rx));

        f.player.play(&track.id, None).unwrap();
        started_rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        assert_eq!(f.player.pause().unwrap().status, PlayerStatus::Loading);
        assert_eq!(f.player.resume().unwrap().status, PlayerStatus::Loading);
        release_tx.send(()).unwrap();

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while f.player.state().status == PlayerStatus::Loading
            && std::time::Instant::now() < deadline
        {
            std::thread::yield_now();
        }
        assert_eq!(f.player.state().status, PlayerStatus::Playing);
        assert_eq!(f.player.state().track.unwrap().id, track.id);
    }

    // @test RF-03 RF-18
    #[test]
    fn navegar_durante_resolucao_descarta_url_antiga() {
        for previous in [false, true] {
            let f = fx(2);
            let track =
                f.db.with(|db| {
                    db.upsert_track(&NewTrack::youtube("pendente", "Pendente", None, 1000, None))
                })
                .unwrap()
                .track;
            let queue = if previous {
                vec![f.ids[0].clone(), track.id.clone()]
            } else {
                vec![track.id.clone(), f.ids[1].clone()]
            };
            let (started_tx, started_rx) = std::sync::mpsc::channel();
            let (release_tx, release_rx) = std::sync::mpsc::channel();
            *f.resolver.stream_gate.lock().unwrap() = Some((started_tx, release_rx));
            f.player.play(&track.id, Some(queue)).unwrap();
            started_rx
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap();

            let selected = if previous {
                f.player.previous().unwrap()
            } else {
                f.player.next().unwrap()
            };
            assert_eq!(selected.status, PlayerStatus::Playing);
            release_tx.send(()).unwrap();
            // Espera a thread antiga encerrar antes de conferir se ela alterou a seleção.
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            while Arc::strong_count(&f.player) > 1 && std::time::Instant::now() < deadline {
                std::thread::yield_now();
            }
            assert_eq!(Arc::strong_count(&f.player), 1);
            assert_eq!(
                f.player.state().track.unwrap().id,
                selected.track.unwrap().id
            );
            assert_eq!(f.player.state().status, PlayerStatus::Playing);
        }
    }

    // @test RF-01
    #[test]
    fn arquivo_ausente_no_disco_devolve_unavailable() {
        let f = fx(1);
        std::fs::remove_file(f.dir.path().join(format!("{}.m4a", f.ids[0]))).unwrap();
        assert_eq!(
            f.player.play(&f.ids[0], None).unwrap_err().code(),
            "unavailable"
        );
    }

    // @test RF-01
    #[test]
    fn faixa_inexistente_devolve_not_found() {
        let f = fx(1);
        assert_eq!(
            f.player.play("nao-existe", None).unwrap_err().code(),
            "not_found"
        );
    }

    // @test RF-01 RF-03
    #[test]
    fn play_carrega_toca_e_monta_o_estado() {
        let f = fx(3);
        let st = f.player.play(&f.ids[1], Some(f.ids.clone())).unwrap();
        assert_eq!(st.status, PlayerStatus::Playing);
        assert_eq!(st.track.as_ref().unwrap().id, f.ids[1]);
        assert_eq!(st.queue, f.ids);
        assert_eq!(st.queue_index, 1);
        assert_eq!(st.duration_ms, 180_000);
        assert!(f.player.current_event_id().is_some());
        // o estado também foi emitido
        let last = f.events.states.lock().unwrap().last().cloned().unwrap();
        assert_eq!(last.queue_index, 1);
        assert_eq!(last.status, PlayerStatus::Playing);
    }

    // @test RF-01 RF-03
    #[test]
    fn play_sem_fila_usa_a_atual_ou_cria_uma() {
        let f = fx(3);
        let st = f.player.play(&f.ids[0], None).unwrap();
        assert_eq!(st.queue, vec![f.ids[0].clone()]);
        let st = f.player.play(&f.ids[2], Some(f.ids.clone())).unwrap();
        assert_eq!(st.queue_index, 2);
        // faixa que está na fila: só muda o índice
        let st = f.player.play(&f.ids[0], None).unwrap();
        assert_eq!(st.queue, f.ids);
        assert_eq!(st.queue_index, 0);
    }

    // @test RF-24
    #[test]
    fn shuffle_reordena_depois_da_atual_persiste_e_off_restaura() {
        let f = fx(8);
        f.player.play(&f.ids[0], Some(f.ids.clone())).unwrap();
        let state = f.player.set_shuffle(ShuffleMode::Random).unwrap();
        assert_eq!(state.queue[0], f.ids[0]);
        let mut sorted = state.queue.clone();
        sorted.sort();
        let mut expected = f.ids.clone();
        expected.sort();
        assert_eq!(sorted, expected);
        assert_eq!(
            f.db.with(|db| db.get_shuffle_mode()).unwrap(),
            ShuffleMode::Random
        );
        let restored = f.player.set_shuffle(ShuffleMode::Off).unwrap();
        assert_eq!(restored.queue, f.ids);
    }

    // @test RF-24
    #[test]
    fn alternar_aleatorio_no_meio_preserva_passado_e_restante() {
        let f = fx(8);
        f.player.play(&f.ids[0], Some(f.ids.clone())).unwrap();
        for _ in 0..3 {
            f.player.next().unwrap();
        }
        let prefix = f.ids[..=3].to_vec();
        let mut remaining = f.ids[4..].to_vec();
        remaining.sort();

        for mode in [
            ShuffleMode::Random,
            ShuffleMode::Off,
            ShuffleMode::Smart,
            ShuffleMode::Random,
            ShuffleMode::Off,
        ] {
            let state = f.player.set_shuffle(mode).unwrap();
            assert_eq!(state.queue_index, 3);
            assert_eq!(state.track.as_ref().unwrap().id, f.ids[3]);
            assert_eq!(&state.queue[..=3], prefix);
            let mut tail = state.queue[4..].to_vec();
            tail.sort();
            assert_eq!(tail, remaining, "restante após {mode:?}");
            if mode == ShuffleMode::Off {
                assert_eq!(state.queue, f.ids);
            }
        }
    }

    // @test RF-24
    #[test]
    fn alternar_apos_avancar_no_aleatorio_preserva_prefixo_tocado() {
        let f = fx(8);
        f.player.play(&f.ids[0], Some(f.ids.clone())).unwrap();
        for _ in 0..3 {
            f.player.next().unwrap();
        }
        f.player.set_shuffle(ShuffleMode::Random).unwrap();
        // Fixa uma permutação possível para a verificação não depender da semente do relógio.
        {
            let mut inner = f.player.lock();
            let mut shuffled = f.ids[..4].to_vec();
            shuffled.extend([
                f.ids[7].clone(),
                f.ids[4].clone(),
                f.ids[5].clone(),
                f.ids[6].clone(),
            ]);
            inner.queue.set_ids(shuffled, 3);
        }
        f.player.next().unwrap();
        let before = f.player.state();
        let prefix = before.queue[..=4].to_vec();
        let mut expected = before.queue[5..].to_vec();
        expected.sort();

        for mode in [ShuffleMode::Smart, ShuffleMode::Off, ShuffleMode::Random] {
            let state = f.player.set_shuffle(mode).unwrap();
            assert_eq!(&state.queue[..=4], prefix);
            assert_eq!(state.queue_index, 4);
            let mut actual = state.queue[5..].to_vec();
            actual.sort();
            assert_eq!(actual, expected);
            if mode == ShuffleMode::Off {
                let restored = f
                    .ids
                    .iter()
                    .filter(|id| expected.contains(id))
                    .cloned()
                    .collect::<Vec<_>>();
                assert_eq!(&state.queue[5..], restored);
            }
        }
    }

    // @test RF-24
    #[test]
    fn id_repetido_na_posicao_atual_preserva_ocorrencias() {
        let f = fx(6);
        let ids = vec![
            f.ids[0].clone(),
            f.ids[1].clone(),
            f.ids[0].clone(),
            f.ids[2].clone(),
            f.ids[3].clone(),
            f.ids[1].clone(),
            f.ids[4].clone(),
            f.ids[5].clone(),
        ];
        f.player.play(&f.ids[0], Some(ids.clone())).unwrap();
        f.player.next().unwrap();
        f.player.next().unwrap();
        for mode in [ShuffleMode::Random, ShuffleMode::Smart, ShuffleMode::Off] {
            let state = f.player.set_shuffle(mode).unwrap();
            assert_eq!(state.queue_index, 2);
            assert_eq!(&state.queue[..=2], &ids[..=2]);
            let mut actual = state.queue[3..].to_vec();
            actual.sort();
            let mut expected = ids[3..].to_vec();
            expected.sort();
            assert_eq!(actual, expected);
            if mode == ShuffleMode::Off {
                assert_eq!(state.queue, ids);
            }
        }
    }

    // @test RF-24
    #[test]
    fn fila_nova_com_aleatorio_poe_a_pedida_primeiro_e_sorteia_todas_as_outras() {
        let f = fx(8);
        f.player.set_shuffle(ShuffleMode::Random).unwrap();
        let state = f.player.play(&f.ids[4], Some(f.ids.clone())).unwrap();
        assert_eq!(state.queue[0], f.ids[4]);
        assert_eq!(state.queue_index, 0);
        let mut sorted = state.queue.clone();
        sorted.sort();
        let mut expected = f.ids.clone();
        expected.sort();
        assert_eq!(sorted, expected, "nenhuma faixa some nem repete");
        let off = f.player.set_shuffle(ShuffleMode::Off).unwrap();
        let mut expected_off = vec![f.ids[4].clone()];
        expected_off.extend(f.ids.iter().filter(|id| *id != &f.ids[4]).cloned());
        assert_eq!(off.queue, expected_off, "off ordena somente o restante");
        assert_eq!(off.queue_index, 0);
    }

    // @test RF-01 RF-13
    #[test]
    fn pausa_nao_conta_como_tempo_ouvido() {
        let f = fx(1);
        f.player.play(&f.ids[0], None).unwrap();
        let ev = f.player.current_event_id().unwrap();
        f.player.process_pending_events();
        f.tick(1000);
        f.player.pause().unwrap();
        f.player.process_pending_events();
        f.tick(30_000); // o relógio parado não avança, mas mesmo assim não deve contar
        f.player.resume().unwrap();
        f.player.process_pending_events();
        f.tick(1000);
        f.player.stop().unwrap();
        let e = f.event(&ev);
        assert_eq!(e.listened_ms, 2000);
        assert!(e.ended_at.is_some());
        assert!(!e.completed && !e.skipped);
    }

    // @test RF-03 RF-13
    #[test]
    fn seek_nao_conta_como_tempo_ouvido() {
        let f = fx(1);
        f.player.play(&f.ids[0], None).unwrap();
        let ev = f.player.current_event_id().unwrap();
        f.player.process_pending_events();
        f.tick(1000);
        let st = f.player.seek(120_000).unwrap();
        assert_eq!(st.position_ms, 120_000);
        f.player.process_pending_events();
        f.tick(1000);
        f.player.stop().unwrap();
        assert_eq!(f.event(&ev).listened_ms, 2000);
    }

    // @test RF-03 RF-13
    #[test]
    fn fim_natural_conclui_e_avanca_a_fila() {
        let f = fx(2);
        f.backend.set_next_duration_ms(10_000);
        f.player.play(&f.ids[0], Some(f.ids.clone())).unwrap();
        let first = f.player.current_event_id().unwrap();
        f.player.process_pending_events();
        f.tick(10_000);
        let e = f.event(&first);
        assert!(e.completed && !e.skipped);
        assert_eq!(e.listened_ms, 10_000);
        let st = f.player.state();
        assert_eq!(st.status, PlayerStatus::Playing);
        assert_eq!(st.queue_index, 1);
        assert_eq!(st.track.unwrap().id, f.ids[1]);
        let second = f.player.current_event_id().unwrap();
        assert_ne!(first, second);
    }

    // @test RF-03 RF-13
    #[test]
    fn fim_da_fila_para_e_mantem_a_ultima() {
        let f = fx(2);
        f.backend.set_next_duration_ms(5_000);
        f.player.play(&f.ids[1], Some(f.ids.clone())).unwrap();
        let ev = f.player.current_event_id().unwrap();
        f.player.process_pending_events();
        f.tick(5_000);
        assert!(f.event(&ev).completed);
        let st = f.player.state();
        assert_eq!(st.status, PlayerStatus::Stopped);
        assert_eq!(st.queue_index, 1);
        assert!(st.track.is_some());
        assert!(f.player.current_event_id().is_none());
        // retomar recomeça a faixa
        let st = f.player.resume().unwrap();
        assert_eq!(st.status, PlayerStatus::Playing);
        assert_eq!(st.position_ms, 0);
    }

    // @test RF-03
    #[test]
    fn track_ended_atrasado_nao_avanca_a_fila() {
        let f = fx(2);
        f.player.play(&f.ids[0], Some(f.ids.clone())).unwrap();
        f.player.handle_event(AudioEvent::TrackEnded); // a faixa atual ainda toca
        assert_eq!(f.player.state().queue_index, 0);
        assert!(f.player.current_event_id().is_some());
    }

    // @test RF-03 RF-13
    #[test]
    fn proxima_e_anterior_marcam_pulo() {
        let f = fx(3);
        f.player.play(&f.ids[0], Some(f.ids.clone())).unwrap();
        let a = f.player.current_event_id().unwrap();
        let st = f.player.next().unwrap();
        assert_eq!(st.queue_index, 1);
        assert!(f.event(&a).skipped);
        let b = f.player.current_event_id().unwrap();
        let st = f.player.previous().unwrap();
        assert_eq!(st.queue_index, 0);
        assert!(f.event(&b).skipped);
        // no começo da fila, "anterior" só reinicia
        let st = f.player.previous().unwrap();
        assert_eq!(st.queue_index, 0);
        assert_eq!(st.position_ms, 0);
    }

    // @test RF-03
    #[test]
    fn anterior_depois_de_3s_reinicia_a_faixa() {
        let f = fx(2);
        f.player.play(&f.ids[1], Some(f.ids.clone())).unwrap();
        f.player.process_pending_events();
        f.tick(5_000);
        let st = f.player.previous().unwrap();
        assert_eq!(st.queue_index, 1);
        assert_eq!(st.position_ms, 0);
    }

    // @test RF-03
    #[test]
    fn proxima_no_fim_da_fila_nao_faz_nada() {
        let f = fx(2);
        f.player.play(&f.ids[1], Some(f.ids.clone())).unwrap();
        let st = f.player.next().unwrap();
        assert_eq!(st.queue_index, 1);
        assert_eq!(st.status, PlayerStatus::Playing);
    }

    // @test RF-03
    #[test]
    fn proxima_pula_faixa_sem_arquivo() {
        let f = fx(3);
        std::fs::remove_file(f.dir.path().join(format!("{}.m4a", f.ids[1]))).unwrap();
        f.player.play(&f.ids[0], Some(f.ids.clone())).unwrap();
        let st = f.player.next().unwrap();
        assert_eq!(st.queue_index, 2);
        assert_eq!(st.track.unwrap().id, f.ids[2]);
    }

    // @test RF-01
    #[test]
    fn falha_ao_carregar_para_o_player_e_devolve_o_erro() {
        let f = fx(2);
        f.player.play(&f.ids[0], None).unwrap();
        f.backend.set_fail_loads(true);
        let err = f.player.play(&f.ids[1], None).unwrap_err();
        assert_eq!(err.code(), "unavailable");
        let st = f.player.state();
        assert_eq!(st.status, PlayerStatus::Stopped);
        assert!(st.track.is_none());
    }

    // @test RF-02
    #[test]
    fn volume_e_mudo_sao_persistidos() {
        let f = fx(1);
        let st = f.player.set_volume(0.35).unwrap();
        assert!((st.volume - 0.35).abs() < 1e-6);
        assert_eq!(f.db.with(|db| db.get_settings()).unwrap().volume, 0.35);
        let st = f.player.set_muted(true).unwrap();
        assert!(st.muted);
        assert!(Prefs::load(&f.dir.path().join("prefs.json")).muted);
        // fora da faixa é limitado; inválido é recusado
        assert_eq!(f.player.set_volume(7.0).unwrap().volume, 1.0);
        assert_eq!(
            f.player.set_volume(f64::NAN).unwrap_err().code(),
            "invalid_input"
        );

        // um novo player aplica o que foi salvo
        let backend2 = Arc::new(NullBackend::new());
        let p2 = Player::new(
            f.db.clone(),
            backend2.clone(),
            Arc::new(crate::downloads::testing::FakeResolver::default()),
            Arc::new(RecordingEvents::default()),
            f.dir.path().join("prefs.json"),
        );
        p2.restore_prefs();
        let s = p2.state();
        assert_eq!(s.volume, 1.0);
        assert!(s.muted);
    }

    // @test RF-03 RF-13
    #[test]
    fn apagar_a_faixa_atual_para_o_player() {
        let f = fx(2);
        f.player.play(&f.ids[0], Some(f.ids.clone())).unwrap();
        f.player.remove_track(&f.ids[0]);
        let st = f.player.state();
        assert_eq!(st.status, PlayerStatus::Stopped);
        assert!(st.track.is_none());
        assert_eq!(st.queue, vec![f.ids[1].clone()]);
    }

    // @test RF-13
    #[test]
    fn shutdown_fecha_o_evento_como_parado() {
        let f = fx(1);
        f.player.play(&f.ids[0], None).unwrap();
        let ev = f.player.current_event_id().unwrap();
        f.player.process_pending_events();
        f.tick(1500);
        f.player.shutdown();
        let e = f.event(&ev);
        assert!(e.ended_at.is_some());
        assert!(!e.completed && !e.skipped);
        assert_eq!(e.listened_ms, 1500);
    }

    // @test RF-01
    #[test]
    fn retomar_sem_nada_carregado_e_erro_claro() {
        let f = fx(1);
        assert_eq!(f.player.resume().unwrap_err().code(), "unavailable");
    }

    // @test RF-01
    #[test]
    fn progresso_e_emitido() {
        let f = fx(1);
        f.player.play(&f.ids[0], None).unwrap();
        f.player.process_pending_events();
        f.tick(1000);
        let p = f.events.progress.lock().unwrap().clone();
        assert_eq!(p.len(), 2);
        assert_eq!(p[1].position_ms, 1000);
    }
}

#[cfg(test)]
mod normalization_tests {
    use super::*;
    use crate::events::testing::RecordingEvents;
    use miupy_audio::NullBackend;
    use miupy_core::{Database, DownloadedFile, NewTrack, Normalization, SettingsPatch};
    // @test RF-28
    #[test]
    fn loading_and_mode_change_apply_gain_without_changing_volume() {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::new(Database::open_in_memory().unwrap());
        let track = db
            .with(|d| d.upsert_track(&NewTrack::youtube("loud", "Música", None, 1000, None)))
            .unwrap()
            .track;
        let file = dir.path().join("song.m4a");
        std::fs::write(&file, b"fake").unwrap();
        db.with(|d| {
            d.mark_downloaded(
                &track.id,
                &DownloadedFile {
                    file_path: file.to_string_lossy().into_owned(),
                    codec: None,
                    bitrate: None,
                    cover_path: None,
                    duration_ms: None,
                },
            )?;
            d.set_loudness(&track.id, -20.0, -2.0)
        })
        .unwrap();
        let backend = Arc::new(NullBackend::new());
        let player = Player::new(
            db.clone(),
            backend.clone(),
            Arc::new(crate::downloads::testing::FakeResolver::default()),
            Arc::new(RecordingEvents::default()),
            dir.path().join("prefs.json"),
        );
        player.play(&track.id, None).unwrap();
        assert_eq!(backend.normalization_gain_db(), 1.0);
        db.with(|d| {
            d.update_settings(&SettingsPatch {
                normalization: Some(Normalization::Loud),
                ..Default::default()
            })
        })
        .unwrap();
        player.refresh_normalization().unwrap();
        assert_eq!(backend.normalization_gain_db(), 9.0);
        assert_eq!(backend.state().volume, 1.0);
    }
}
