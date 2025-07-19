use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error("Not Found")]
    NotFound,
    #[error("Conflict")]
    Conflict,
}
