use crate::db::DatabaseRepo;
use crate::hash::{CommitHash, MigrationHash};
use crate::traits::{Repo, Commit};

#[derive(Debug, Clone)]
pub struct Plan {
    pub local_head: CommitHash,
    pub db_head: CommitHash,
    pub merge_base: CommitHash,
    pub rollbacks: Vec<RollbackStep>,
    pub migrations: Vec<MigrationStep>,
}

impl Plan {
    pub fn is_empty(&self) -> bool {
        let heads_eq = self.local_head == self.db_head;
        let vec_empty = self.rollbacks.is_empty() && self.migrations.is_empty();
        assert_eq!(heads_eq, vec_empty);
        heads_eq
    }

    pub fn is_fast_forward(&self) -> bool {
        self.rollbacks.is_empty()
    }

    pub fn verify(&self) -> Result<(), String> {
        let mut head = self.db_head.clone();
        for (i, rollback) in self.rollbacks.iter().enumerate() {
            if rollback.commit() != head {
                return Err(format!("Rollback {i} (from {:?}) does not cleanly apply!", rollback.source));
            }
            head = rollback.parent.clone();
        }
        if head != self.merge_base {
            return Err(format!("Merge base not reached!"));
        }
        for (i, migration) in self.migrations.iter().enumerate() {
            if migration.parent != head {
                return Err(format!("Migration {i} does not cleanly apply!"));
            }
            head = migration.commit();
        }
        if head != self.local_head {
            return Err(format!("Local head not reached!"));
        }

        Ok(())
    }

    pub fn from_db_and_local<Local: Repo>(db: &mut DatabaseRepo, local: &mut Local) -> Result<Self, PlanError> {
        let db_head = db.head();
        let local_head = local.head();

        let (merge_base, rollbacks) = if db_head.is_zero() || local.commit(&db_head).is_some() {
            (db_head.clone(), vec![])
        } else {
            let mut rollbacks = vec![];
            let mut head = db_head.clone();

            while local.commit(&head).is_none() {
                if head.is_zero() {
                    break;
                }

                let commit = db.commit(&head).ok_or_else(|| PlanError::MissingCommit(RepoSource::Database, head))?;
                head = commit.parent();

                let hash = commit.migration_hash();

                match db.rollback(&hash) {
                    Some(rollback) => {
                        rollbacks.push(RollbackStep {
                            target: hash,
                            source: RepoSource::Database,
                            text: rollback,
                            parent: commit.parent(),
                        });
                    }
                    None => {
                        match local.rollback(&hash) {
                            Some(rollback) => {
                                rollbacks.push(RollbackStep {
                                    target: hash,
                                    source: RepoSource::Local,
                                    text: rollback,
                                    parent: commit.parent(),
                                });
                            }
                            None => return Err(PlanError::MissingRollback(hash)),
                        }
                    }
                }
            }

            (head, rollbacks)
        };

        let mut migrations = vec![];
        let mut head = local_head.clone();

        while head != merge_base {
            let commit = local.commit(&head).ok_or_else(|| PlanError::MissingCommit(RepoSource::Local, head))?;
            head = commit.parent();
            migrations.push(MigrationStep {
                parent: commit.parent(),
                name: commit.migration_name(),
                text: commit.migration_text(),
                rollback: local.rollback(&commit.migration_hash()),
            });
        }

        migrations.reverse();

        Ok(Plan {
            local_head,
            db_head,
            merge_base,
            rollbacks,
            migrations,
        })
    }
}

#[derive(Debug)]
pub enum PlanError {
    MissingCommit(RepoSource, CommitHash),
    MissingRollback(MigrationHash),
}

impl std::fmt::Display for PlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            PlanError::MissingCommit(RepoSource::Database, commit) => write!(f, "Unable to find commit in database: {commit}"),
            PlanError::MissingCommit(RepoSource::Local, commit) => write!(f, "Unable to find commit locally: {commit}"),
            PlanError::MissingRollback(hash) => write!(f, "Unable to find rollback for migration: {hash}"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RollbackStep {
    pub target: MigrationHash,
    pub source: RepoSource,
    pub text: String,
    pub parent: CommitHash,
}

impl RollbackStep {
    pub fn commit(&self) -> CommitHash {
        CommitHash::from_parent_and_hash(&self.parent, &self.target)
    }
}

#[derive(Debug, Clone)]
pub enum RepoSource {
    Database,
    Local,
}

#[derive(Debug, Clone)]
pub struct MigrationStep {
    pub parent: CommitHash,
    pub name: String,
    pub text: String,
    pub rollback: Option<String>,
}

impl MigrationStep {
    pub fn hash(&self) -> MigrationHash {
        MigrationHash::from_name_and_text(&self.name, &self.text)
    }

    pub fn commit(&self) -> CommitHash {
        CommitHash::from_parent_and_hash(&self.parent, &self.hash())
    }
}
