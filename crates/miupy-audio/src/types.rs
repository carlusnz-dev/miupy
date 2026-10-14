//! Tipos públicos compartilhados: estado, eventos e a trait [`AudioBackend`].

use crate::error::Result;
use crossbeam_channel::Receiver;
use std::path::{Path, PathBuf};

/// Intervalo entre eventos de progresso enquanto toca.
pub const PROGRESS_INTERVAL_MS: u64 = 500;

/// Situação do player. Espelha `PlayerStatus` do contrato IPC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackStatus {
    /// Nada carregado (inicial, após `stop` ou após o fim da faixa).
    Stopped,
    /// Abrindo/decodificando o arquivo.
    Loading,
    Playing,
    /// Faixa carregada, parada (logo após `load`, ou após `pause`).
    Paused,
}

/// Foto do estado do motor.
#[derive(Debug, Clone, PartialEq)]
pub struct PlaybackState {
    pub status: PlaybackStatus,
    /// Faixa carregada, se houver.
    pub path: Option<PathBuf>,
    /// URL da faixa em streaming, se houver.
    pub url: Option<String>,
    pub position_ms: u64,
    /// Duração total; `0` enquanto desconhecida.
    pub duration_ms: u64,
    /// Volume do usuário, `0.0..=1.0` (a curva perceptual é aplicada internamente).
    pub volume: f32,
    pub muted: bool,
}

impl Default for PlaybackState {
    fn default() -> Self {
        Self {
            status: PlaybackStatus::Stopped,
            path: None,
            url: None,
            position_ms: 0,
            duration_ms: 0,
            volume: 1.0,
            muted: false,
        }
    }
}

/// Dados conhecidos pela casca antes de iniciar o streaming.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StreamOptions {
    /// Cabeçalhos HTTP (por exemplo, User-Agent e Referer).
    pub headers: Vec<(String, String)>,
    /// Duração da faixa em milissegundos; zero quando desconhecida.
    pub duration_ms: u64,
}

/// Evento publicado pelo backend.
#[derive(Debug, Clone, PartialEq)]
pub enum AudioEvent {
    /// Mudou status, faixa, duração, volume ou mudo (ou houve seek).
    StateChanged(PlaybackState),
    /// A cada ~500 ms enquanto toca.
    Progress { position_ms: u64, duration_ms: u64 },
    /// A faixa terminou **sozinha**. Vem logo após um `StateChanged` com
    /// `Stopped`. Nunca é emitido por `stop()` nem por um novo `load()`: é este
    /// evento que dispara o avanço da fila e o fechamento do registro de reprodução.
    TrackEnded,
    /// Falha assíncrona (ex.: erro de decodificação no meio da faixa).
    Error { message: String },
}

/// Motor de reprodução de uma faixa por vez. Fila, próxima/anterior e banco
/// ficam na camada acima.
///
/// Todos os métodos usam `&self` (sincronização interna), então o backend pode
/// viver em `Arc<dyn AudioBackend>` ou `Box<dyn AudioBackend>`.
pub trait AudioBackend: Send + Sync {
    /// Carrega o arquivo, substituindo a faixa atual (sem emitir `TrackEnded`).
    /// Ao retornar, o status é `Paused` na posição 0; chame [`play`](Self::play).
    fn load(&self, path: &Path) -> Result<()>;
    /// Inicia a abertura assíncrona de uma URL HTTP. Retorna antes de acessar a rede;
    /// `Loading` permanece até o áudio estar pronto. Falhas chegam por `AudioEvent::Error`.
    // @req RF-18
    fn load_url(&self, url: &str, options: StreamOptions) -> Result<()>;
    /// Inicia a faixa carregada (equivale a `resume`).
    fn play(&self) -> Result<()>;
    fn pause(&self) -> Result<()>;
    fn resume(&self) -> Result<()>;
    /// Descarrega a faixa. Não emite `TrackEnded`.
    fn stop(&self) -> Result<()>;
    /// Vai para `position_ms` (limitado à duração, se conhecida).
    fn seek(&self, position_ms: u64) -> Result<()>;
    /// Volume do usuário; valores fora de `0.0..=1.0` são limitados.
    fn set_volume(&self, volume: f32) -> Result<()>;
    // @req RF-28
    fn set_normalization_gain_db(&self, gain_db: f32) -> Result<()>;
    fn set_muted(&self, muted: bool) -> Result<()>;
    /// Estado atual (não bloqueia).
    fn state(&self) -> PlaybackState;
    /// Receptor dos eventos. Cada chamada devolve um clone do mesmo canal:
    /// com vários receptores, cada evento chega a apenas um deles. Use um só
    /// consumidor (na prática, o loop que repassa para `emit` do Tauri).
    fn events(&self) -> Receiver<AudioEvent>;
}

/// Curva perceptual: ganho de amplitude = volume². Metade do cursor dá -12 dB,
/// em vez dos -6 dB de uma escala linear.
// @req RF-02
pub fn volume_to_gain(volume: f32) -> f32 {
    let v = volume.clamp(0.0, 1.0);
    v * v
}
