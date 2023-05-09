use crate::hash::{CommitHash, MigrationHash};

pub trait Repo {
    type Commit: Commit;
    fn head(&mut self) -> CommitHash;
    fn commit(&mut self, commit: &CommitHash) -> Option<Self::Commit>;
    fn rollback(&mut self, hash: &MigrationHash) -> Option<String>;
}

pub trait Commit {
    fn commit_hash(&self) -> CommitHash;
    fn parent(&self) -> CommitHash;
    fn migration_name(&self) -> String;
    fn migration_text(&self) -> String;
    fn migration_hash(&self) -> MigrationHash;
}
