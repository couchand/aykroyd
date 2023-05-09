pub mod db;
pub mod embedded;
pub mod fs;
pub mod hash;
pub mod local;
pub mod plan;
pub mod traits;

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    detail: Option<String>,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &self.kind {
            ErrorKind::InvalidHash => write!(
                f,
                "invalid hash: {}",
                self.detail.as_ref().cloned().unwrap_or_default()
            ),
        }
    }
}

impl std::error::Error for Error {}

impl Error {
    fn invalid_hash(detail: &str) -> Self {
        Error {
            kind: ErrorKind::InvalidHash,
            detail: Some(detail.into()),
        }
    }
}

#[derive(Debug)]
enum ErrorKind {
    InvalidHash,
}
