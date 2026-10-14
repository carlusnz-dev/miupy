//! Erro único da casca. Serializa como `{ code, message }` (contrato de `ipc.md`) e converte
//! os erros das três crates, de modo que os comandos usem só `?`.

use miupy_audio::AudioError;
use miupy_media::MediaError;
use serde::{Serialize, Serializer, ser::SerializeStruct};

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error(transparent)]
    Core(#[from] miupy_core::Error),
    #[error(transparent)]
    Media(#[from] MediaError),
    #[error(transparent)]
    Audio(#[from] AudioError),
    #[error("erro de E/S: {0}")]
    Io(#[from] std::io::Error),
    #[error("{message}")]
    Other { code: &'static str, message: String },
}

impl AppError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        AppError::Other {
            code,
            message: message.into(),
        }
    }
    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::new("unavailable", message)
    }
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new("not_found", message)
    }
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new("invalid_input", message)
    }
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new("internal", message)
    }

    /// Código do contrato de IPC (`not_found`, `invalid_input`, `conflict`, `io`, `network`,
    /// `unavailable`, `internal`).
    pub fn code(&self) -> &'static str {
        match self {
            AppError::Core(e) => e.code(),
            // `Cancelled` não existe no contrato: quem chega aqui com ele é um bug de fluxo.
            AppError::Media(MediaError::Cancelled) => "internal",
            AppError::Media(e) => e.code(),
            AppError::Audio(e) => match e {
                AudioError::Io { .. } => "io",
                AudioError::Network { .. } => "network",
                AudioError::NoOutputDevice(_)
                | AudioError::Decode { .. }
                | AudioError::NothingLoaded
                | AudioError::Unsupported(_) => "unavailable",
                AudioError::Seek(_) | AudioError::EngineStopped => "internal",
            },
            AppError::Io(_) => "io",
            AppError::Other { code, .. } => code,
        }
    }

    /// O download/operação foi cancelado pelo usuário (não é erro para ele).
    pub fn is_cancelled(&self) -> bool {
        matches!(self, AppError::Media(MediaError::Cancelled))
    }

    /// Erro específico de uma faixa (sem arquivo, arquivo ausente ou ilegível, faixa apagada):
    /// o avanço automático da fila pula a faixa e tenta a seguinte. Falhas do dispositivo de
    /// áudio não entram aqui (pular não adiantaria).
    pub fn is_track_specific(&self) -> bool {
        match self {
            AppError::Core(miupy_core::Error::NotFound(_)) => true,
            AppError::Audio(AudioError::Decode { .. } | AudioError::Io { .. }) => true,
            AppError::Other { code, .. } => *code == "unavailable" || *code == "not_found",
            _ => false,
        }
    }
}

impl From<tauri::Error> for AppError {
    fn from(e: tauri::Error) -> Self {
        AppError::internal(e.to_string())
    }
}

impl From<tokio::task::JoinError> for AppError {
    fn from(e: tokio::task::JoinError) -> Self {
        AppError::internal(format!("tarefa interrompida: {e}"))
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut st = s.serialize_struct("AppError", 2)?;
        st.serialize_field("code", self.code())?;
        st.serialize_field("message", &self.to_string())?;
        st.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // @test RNF-01
    #[test]
    fn serializa_como_code_e_message() {
        let v = serde_json::to_value(AppError::unavailable("sem arquivo")).unwrap();
        assert_eq!(
            v,
            serde_json::json!({ "code": "unavailable", "message": "sem arquivo" })
        );
    }

    // @test RNF-01
    #[test]
    fn converte_erros_das_crates_para_codigos_do_contrato() {
        let nf: AppError = miupy_core::Error::NotFound("faixa x".into()).into();
        assert_eq!(nf.code(), "not_found");
        let cf: AppError = miupy_core::Error::Conflict("dup".into()).into();
        assert_eq!(cf.code(), "conflict");
        assert_eq!(
            AppError::from(MediaError::Network("x".into())).code(),
            "network"
        );
        assert_eq!(
            AppError::from(MediaError::Unavailable("x".into())).code(),
            "unavailable"
        );
        assert_eq!(AppError::from(MediaError::Cancelled).code(), "internal");
        let dec = AudioError::Decode {
            path: PathBuf::from("/a.m4a"),
            message: "ruim".into(),
        };
        assert_eq!(AppError::from(dec).code(), "unavailable");
        assert_eq!(
            AppError::from(AudioError::NoOutputDevice("nenhum".into())).code(),
            "unavailable"
        );
        assert_eq!(AppError::from(std::io::Error::other("x")).code(), "io");
    }

    // @test RF-03
    #[test]
    fn so_erros_de_faixa_pulam_na_fila() {
        assert!(AppError::unavailable("sem arquivo").is_track_specific());
        assert!(AppError::from(miupy_core::Error::NotFound("x".into())).is_track_specific());
        assert!(!AppError::from(AudioError::NoOutputDevice("x".into())).is_track_specific());
        assert!(!AppError::internal("x").is_track_specific());
    }

    // @test RF-10
    #[test]
    fn cancelamento_e_reconhecido() {
        assert!(AppError::from(MediaError::Cancelled).is_cancelled());
        assert!(!AppError::from(MediaError::Network("x".into())).is_cancelled());
    }
}
