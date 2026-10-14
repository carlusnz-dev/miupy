//! Erro único da crate.

use serde::{Serialize, Serializer, ser::SerializeStruct};

pub type Result<T> = std::result::Result<T, Error>;

/// Erro de qualquer operação do núcleo. [`Error::code`] devolve um dos códigos de
/// `docs/arquitetura/ipc.md`; a camada Tauri serializa o erro como `{ code, message }`.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("não encontrado: {0}")]
    NotFound(String),
    #[error("entrada inválida: {0}")]
    InvalidInput(String),
    #[error("conflito: {0}")]
    Conflict(String),
    #[error("erro de E/S: {0}")]
    Io(#[from] std::io::Error),
    /// Banco travado por outra conexão por mais que o `busy_timeout`.
    #[error("banco indisponível: {0}")]
    Unavailable(String),
    #[error("erro no banco de dados: {0}")]
    Database(rusqlite::Error),
    #[error("falha ao migrar o banco: {0}")]
    Migration(#[from] rusqlite_migration::Error),
    #[error("erro interno: {0}")]
    Internal(String),
}

impl Error {
    // @req RNF-01
    /// Código estável do contrato de IPC: `not_found`, `invalid_input`, `conflict`, `io`,
    /// `unavailable` ou `internal`. (`network` não existe aqui: o núcleo não usa rede.)
    pub fn code(&self) -> &'static str {
        match self {
            Error::NotFound(_) => "not_found",
            Error::InvalidInput(_) => "invalid_input",
            Error::Conflict(_) => "conflict",
            Error::Io(_) => "io",
            Error::Unavailable(_) => "unavailable",
            Error::Database(_) | Error::Migration(_) | Error::Internal(_) => "internal",
        }
    }

    pub(crate) fn not_found(what: &str, id: &str) -> Self {
        Error::NotFound(format!("{what} {id}"))
    }

    pub(crate) fn invalid(msg: impl Into<String>) -> Self {
        Error::InvalidInput(msg.into())
    }
}

impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        use rusqlite::{Error as E, ErrorCode as C, ffi};
        let mapped = match &e {
            E::QueryReturnedNoRows => Some(Error::NotFound("registro não encontrado".into())),
            E::SqliteFailure(f, msg) => {
                let text = msg.clone().unwrap_or_else(|| f.to_string());
                match f.code {
                    C::ConstraintViolation => Some(match f.extended_code {
                        ffi::SQLITE_CONSTRAINT_UNIQUE | ffi::SQLITE_CONSTRAINT_PRIMARYKEY => {
                            Error::Conflict(text)
                        }
                        ffi::SQLITE_CONSTRAINT_FOREIGNKEY => {
                            Error::InvalidInput("referência a um registro inexistente".into())
                        }
                        _ => Error::InvalidInput(text),
                    }),
                    C::DatabaseBusy | C::DatabaseLocked => Some(Error::Unavailable(text)),
                    _ => None,
                }
            }
            _ => None,
        };
        mapped.unwrap_or(Error::Database(e))
    }
}

impl Serialize for Error {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        let mut st = s.serialize_struct("Error", 2)?;
        st.serialize_field("code", self.code())?;
        st.serialize_field("message", &self.to_string())?;
        st.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // @test RNF-01
    #[test]
    fn codigos_seguem_o_contrato_do_ipc() {
        assert_eq!(Error::NotFound("x".into()).code(), "not_found");
        assert_eq!(Error::InvalidInput("x".into()).code(), "invalid_input");
        assert_eq!(Error::Conflict("x".into()).code(), "conflict");
        assert_eq!(Error::Unavailable("x".into()).code(), "unavailable");
        assert_eq!(Error::Internal("x".into()).code(), "internal");
        let io = Error::from(std::io::Error::other("falhou"));
        assert_eq!(io.code(), "io");
        let sem_linhas = Error::from(rusqlite::Error::QueryReturnedNoRows);
        assert_eq!(sem_linhas.code(), "not_found");
    }

    // @test RNF-01
    #[test]
    fn serializa_como_code_e_message() {
        let v = serde_json::to_value(Error::NotFound("faixa 1".into())).unwrap();
        assert_eq!(v["code"], "not_found");
        assert_eq!(v["message"], "não encontrado: faixa 1");
    }
}
