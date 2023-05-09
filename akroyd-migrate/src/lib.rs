pub mod db;
pub mod db2;
pub mod embedded;
pub mod embedded2;
pub mod fs;
pub mod hash;
pub mod hash2;
pub mod local;
pub mod local2;
pub mod plan;
pub mod traits;

/*
pub struct Migration {
    pub name: String,
    pub text: String,
}

impl Migration {
    pub fn hash(&self) -> MigrationHash {
        todo!()
    }
}

pub struct Commit {
    pub parent: CommitHash,
    pub migration: MigrationHash,
}

impl Commit {
    pub fn hash(&self) -> CommitHash {
        todo!()
    }
}

pub struct Rollback {
    pub target: MigrationHash,
    pub text: String,
}

pub struct Link {
    commit: Commit,
    rollback: Option<Rollback>, // rollback.target == commit.migration
}

pub struct Chain {
    links: Vec<Link>, // links[n].parent == links[n+1].hash()
}

impl Chain {
    pub fn get_hash(&self, index: usize) -> CommitHash {
        self.links.get(index).map(|link| link.commit.hash()).unwrap_or_default()
    }

    pub fn get_rollbac(&self, index: usize) -> Option<&Rollback> {
        self.links.get(index).and_then(|link| link.rollback.as_ref())
    }
}

pub trait MigrationRepository {
    fn get_migration(&self, hash: MigrationHash) -> Option<Migration>; // result.hash() == hash
    fn get_commit(&self, hash: CommitHash) -> Option<Commit>; // result.hash() == hash
    fn get_rollback(&self, hash: MigrationHash) -> Option<Rollback>; // result.target == hash
    fn get_head(&self) -> CommitHash;
}
*/

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
