//! Erros do motor de áudio.

use std::path::PathBuf;

/// Erros devolvidos pelas operações de um [`AudioBackend`](crate::AudioBackend).
#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    /// Não há dispositivo de saída de áudio utilizável (ou ele não abriu).
    #[error("nenhum dispositivo de saída de áudio disponível: {0}")]
    NoOutputDevice(String),
    /// Não foi possível ler o arquivo de áudio.
    #[error("não foi possível abrir {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// Formato ausente, não suportado ou arquivo corrompido.
    #[error("não foi possível decodificar {path}: {message}")]
    Decode { path: PathBuf, message: String },
    /// Falha de rede ao abrir ou ler uma faixa em streaming.
    #[error("falha de rede ao tocar {url}: {message}")]
    Network { url: String, message: String },
    /// Operação que exige uma faixa carregada, mas nenhuma está.
    #[error("nenhuma faixa carregada")]
    NothingLoaded,
    /// O decoder não aceitou o seek.
    #[error("seek não suportado ou falhou: {0}")]
    Seek(String),
    /// A thread do motor terminou; o backend não é mais utilizável.
    #[error("o motor de áudio foi encerrado")]
    EngineStopped,
    /// Backend não disponível nesta plataforma (ex.: Android sem o plugin Media3).
    #[error("backend de áudio indisponível nesta plataforma: {0}")]
    Unsupported(String),
}

/// Atalho de `Result` da crate.
pub type Result<T> = std::result::Result<T, AudioError>;
