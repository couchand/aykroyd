pub mod db;
pub mod embedded;
pub mod fs;
pub mod hash;
pub mod local;
pub mod plan;
pub mod source;
pub mod traits;

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    detail: Option<String>,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let detail = self.detail.as_ref().cloned().unwrap_or_default();
        match &self.kind {
            ErrorKind::InvalidHash => write!(f, "invalid hash: {detail}"),
            ErrorKind::Planning => write!(f, "planning error: {detail}"),
            ErrorKind::Db => write!(f, "db repo error: {detail}"),
            ErrorKind::Fs => write!(f, "fs repo error: {detail}"),
            ErrorKind::Divergence => write!(f, "unable to fast-forward: {detail}"),
            ErrorKind::MultipleHeads => write!(f, "multiple heads: {detail}"),
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

    fn divergence(detail: &str) -> Self {
        Error {
            kind: ErrorKind::Divergence,
            detail: Some(detail.into()),
        }
    }

    fn multiple_heads(detail: &str) -> Self {
        Error {
            kind: ErrorKind::MultipleHeads,
            detail: Some(detail.into()),
        }
    }
}

#[derive(Debug)]
enum ErrorKind {
    InvalidHash,
    Planning,
    Db,
    Fs,
    Divergence,
    MultipleHeads,
}

impl From<plan::PlanError> for Error {
    fn from(err: plan::PlanError) -> Self {
        Error {
            kind: ErrorKind::Planning,
            detail: Some(err.to_string()),
        }
    }
}

impl From<tokio_postgres::Error> for Error {
    fn from(err: tokio_postgres::Error) -> Self {
        Error {
            kind: ErrorKind::Db,
            detail: Some(err.to_string()),
        }
    }
}

impl From<fs::CheckError> for Error {
    fn from(err: fs::CheckError) -> Self {
        Error {
            kind: ErrorKind::Fs,
            detail: Some(err.to_string()),
        }
    }
}
