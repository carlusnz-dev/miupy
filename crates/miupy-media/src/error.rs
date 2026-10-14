//! Erros da crate. `code()` segue os códigos de `docs/arquitetura/ipc.md`.

use std::io;

/// Resultado padrão da crate.
pub type Result<T> = std::result::Result<T, MediaError>;

/// Erro tipado de qualquer operação da crate. Nenhum caminho de rede ou de processo dá `panic`.
#[derive(Debug, thiserror::Error)]
pub enum MediaError {
    /// Falha de rede, timeout, resposta HTTP inesperada ou de limite de taxa.
    #[error("falha de rede: {0}")]
    Network(String),
    /// Ferramenta externa ausente (ex.: `yt-dlp` não instalado) ou formato indisponível.
    #[error("recurso indisponível: {0}")]
    Unavailable(String),
    /// Vídeo, gravação, capa ou letra inexistente.
    #[error("não encontrado: {0}")]
    NotFound(String),
    /// Falha de leitura ou escrita em disco.
    #[error("erro de E/S: {0}")]
    Io(String),
    /// Entrada rejeitada antes de qualquer chamada (consulta vazia, id malformado...).
    #[error("entrada inválida: {0}")]
    InvalidInput(String),
    /// Operação cancelada pelo chamador.
    #[error("operação cancelada")]
    Cancelled,
    /// Saída inesperada de ferramenta externa, arquivo ilegível, bug.
    #[error("erro interno: {0}")]
    Internal(String),
}

impl MediaError {
    /// Código estável para a camada de IPC. `Cancelled` não existe no contrato do `ipc.md`: a
    /// camada Tauri deve tratá-lo antes (cancelar não é erro para o usuário).
    pub fn code(&self) -> &'static str {
        match self {
            Self::Network(_) => "network",
            Self::Unavailable(_) => "unavailable",
            Self::NotFound(_) => "not_found",
            Self::Io(_) => "io",
            Self::InvalidInput(_) => "invalid_input",
            Self::Cancelled => "cancelled",
            Self::Internal(_) => "internal",
        }
    }
}

impl From<io::Error> for MediaError {
    fn from(e: io::Error) -> Self {
        if e.kind() == io::ErrorKind::NotFound {
            Self::NotFound(e.to_string())
        } else {
            Self::Io(e.to_string())
        }
    }
}

impl From<reqwest::Error> for MediaError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_decode() {
            Self::Internal(format!("resposta em formato inesperado: {e}"))
        } else {
            // `without_url` evita vazar a URL completa (consultas do usuário) nas mensagens.
            Self::Network(e.without_url().to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // @test RF-09
    #[test]
    fn codigos_seguem_o_contrato() {
        assert_eq!(MediaError::Network("x".into()).code(), "network");
        assert_eq!(MediaError::Unavailable("x".into()).code(), "unavailable");
        assert_eq!(MediaError::NotFound("x".into()).code(), "not_found");
        assert_eq!(MediaError::Io("x".into()).code(), "io");
        assert_eq!(MediaError::InvalidInput("x".into()).code(), "invalid_input");
        assert_eq!(MediaError::Internal("x".into()).code(), "internal");
    }

    // @test RF-10
    #[test]
    fn io_not_found_vira_not_found() {
        let e: MediaError = io::Error::from(io::ErrorKind::NotFound).into();
        assert_eq!(e.code(), "not_found");
        let e: MediaError = io::Error::from(io::ErrorKind::PermissionDenied).into();
        assert_eq!(e.code(), "io");
    }
}
