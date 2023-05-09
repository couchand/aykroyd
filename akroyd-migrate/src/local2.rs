use crate::hash2::{CommitHash, MigrationHash};
use crate::traits::{Commit, Repo};

#[derive(Debug, Clone)]
pub struct LocalRepo {
    pub head: CommitHash,
    pub commits: Vec<LocalCommit>,
}

impl Repo for LocalRepo {
    type Commit = LocalCommit;
    fn head(&self) -> CommitHash {
        self.head.clone()
    }

    fn commit(&self, commit: &CommitHash) -> Option<Self::Commit> {
        self.commits
            .iter()
            .find(|c| c.commit_hash() == *commit)
            .cloned()
    }

    fn rollback(&self, hash: &MigrationHash) -> Option<String> {
        self.commits
            .iter()
            .find(|c| c.migration_hash() == *hash)
            .and_then(|c| c.rollback_text.clone())
    }
}

#[derive(Debug, Clone)]
pub struct LocalCommit {
    pub parent: CommitHash,
    pub name: String,
    pub migration_text: String,
    pub rollback_text: Option<String>,
}

impl Commit for LocalCommit {
    fn commit_hash(&self) -> CommitHash {
        CommitHash::from_parent_and_hash(&self.parent, &self.migration_hash())
    }

    fn parent(&self) -> CommitHash {
        self.parent.clone()
    }

    fn migration_name(&self) -> String {
        self.name.clone()
    }

    fn migration_text(&self) -> String {
        self.migration_text.clone()
    }

    fn migration_hash(&self) -> MigrationHash {
        MigrationHash::from_name_and_text(&self.name, &self.migration_text)
    }
}
