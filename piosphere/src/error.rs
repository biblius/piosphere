use thiserror::Error;

use crate::socket::PiosphereIOError;

#[derive(Debug, Error)]
pub enum PiosphereError {
    #[error("{0}")]
    IO(#[from] std::io::Error),

    #[error("{0}")]
    NginxParse(String),

    #[error("{0}")]
    PiosphereIO(#[from] PiosphereIOError),

    #[error("{0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("{0}")]
    Bincode(#[from] bincode::Error),
}
