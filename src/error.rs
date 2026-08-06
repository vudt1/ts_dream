//! Shared error types.

use thiserror::Error;

/// Errors surfaced across the port. Grouped by subsystem so callers can
/// decide whether to hard-exit (DB / migration / data load) or swallow
/// (handler exceptions, which the C# server silently ignores).
#[derive(Debug, Error)]
pub enum TsError {
    #[error("config error: {0}")]
    Config(String),

    #[error("protocol/framing error: {0}")]
    Protocol(String),

    #[error("data load error: {0}")]
    Data(String),

    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),

    #[error("migration error: {0}")]
    Migrate(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, TsError>;
