pub mod db;
pub mod hash;
pub mod local;

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
            ErrorKind::UnableToFixDownTree => write!(
                f,
                "missing down tree refs: {}",
                self.detail.as_ref().cloned().unwrap_or_default()
            ),
            ErrorKind::Io(e) => write!(f, "unhandled i/o error: {e}"),
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

    fn unable_to_fix_down_tree(detail: &str) -> Self {
        Error {
            kind: ErrorKind::UnableToFixDownTree,
            detail: Some(detail.into()),
        }
    }

    fn io_error(error: std::io::Error) -> Self {
        Error {
            kind: ErrorKind::Io(error),
            detail: None,
        }
    }
}

#[derive(Debug)]
enum ErrorKind {
    InvalidHash,
    UnableToFixDownTree,
    Io(std::io::Error), // This variant is terrible and should be removed.  Handle the kinds!
}
