use thiserror::Error;

#[derive(Debug, Error)]
pub enum PiteriaError {
    #[error("{0}")]
    NginxParse(String),
}
