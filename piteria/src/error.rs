use thiserror::Error;

use crate::socket::PiteriaIOError;

#[derive(Debug, Error)]
pub enum PiteriaError {
    #[error("{0}")]
    IO(#[from] std::io::Error),

    #[error("{0}")]
    NginxParse(String),

    #[error("{0}")]
    PiteriaIO(#[from] PiteriaIOError),

    #[error("{0}")]
    Sqlx(#[from] sqlx::Error),
}
