use crate::hash2::{CommitHash, MigrationHash};

pub trait Repo {
    type Commit: Commit;
    fn head(&self) -> CommitHash;
    fn commit(&self, commit: &CommitHash) -> Option<Self::Commit>;
    fn rollback(&self, hash: &MigrationHash) -> Option<String>;
}

pub trait Commit {
    fn commit_hash(&self) -> CommitHash;
    fn parent(&self) -> CommitHash;
    fn migration_name(&self) -> String;
    fn migration_text(&self) -> String;
    fn migration_hash(&self) -> MigrationHash;
}
