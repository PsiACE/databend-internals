use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("minibend io error: {0}")]
    IO(#[from] std::io::Error),
    #[error("minibend arrow2 error: {0}")]
    Arrow(#[from] arrow2::error::Error),
    #[error("minibend no such table error: {0}")]
    NoSuchTable(String),
}

impl From<String> for Error {
    fn from(v: String) -> Self {
        Self::NoSuchTable(v)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
