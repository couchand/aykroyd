use crate::hash2::{CommitHash, MigrationHash};

pub struct DbRepo;

impl DbRepo {
    fn head(&self) -> CommitHash {
        todo!()
    }

    fn commit(&self, commit: &CommitHash) -> Option<DbCommit> {
        todo!()
    }

    fn rollback(&self, hash: &MigrationHash) -> Option<String> {
        todo!()
    }
}

pub struct DbCommit;

impl DbCommit {
    fn parent(&self) -> CommitHash {
        todo!()
    }

    fn migration_hash(&self) -> MigrationHash {
        todo!()
    }
}

pub trait LocalRepo {
    type Commit: LocalCommit;
    fn head(&self) -> CommitHash;
    fn commit(&self, commit: &CommitHash) -> Option<Self::Commit>;
    fn rollback(&self, hash: &MigrationHash) -> Option<String>;
}

pub trait LocalCommit {
    fn parent(&self) -> CommitHash;
    fn migration_name(&self) -> String;
    fn migration_text(&self) -> String;
    fn migration_hash(&self) -> MigrationHash;
}

impl<'a> LocalRepo for &'a crate::fs::FsRepo {
    type Commit = FsCommit<'a>;
    fn head(&self) -> CommitHash {
        match self.head_name() {
            None => CommitHash::default(),
            Some(head) => self.migration(head).unwrap().unwrap().commit().unwrap(),
        }
    }
    fn commit(&self, commit: &CommitHash) -> Option<Self::Commit> {
        self.migrations()
            .unwrap()
            .into_iter()
            .filter(|migration| migration.is_committed().unwrap())
            .find(|migration| migration.commit().unwrap() == *commit)
            .map(|migration| FsCommit { migration, repo: *self })
    }
    fn rollback(&self, hash: &MigrationHash) -> Option<String> {
        self.migrations()
            .unwrap()
            .into_iter()
            .filter(|migration| migration.is_committed().unwrap())
            .find(|migration| migration.hash().unwrap() == *hash)
            .and_then(|migration| migration.rollback_text().unwrap().into())
    }
}

pub struct FsCommit<'a> {
    migration: crate::fs::FsMigration,
    repo: &'a crate::fs::FsRepo,
}

impl<'a> LocalCommit for FsCommit<'a> {
    fn parent(&self) -> CommitHash {
        match self.migration.parent_name().unwrap() {
            None => CommitHash::default(),
            Some(parent) => self.repo.migration(parent).unwrap().unwrap().commit().unwrap(),
        }
    }

    fn migration_name(&self) -> String {
        self.migration.name().to_string()
    }

    fn migration_text(&self) -> String {
        self.migration.migration_text().unwrap().unwrap_or_default()
    }

    fn migration_hash(&self) -> MigrationHash {
        self.migration.hash().unwrap()
    }
}

#[derive(Debug, Clone)]
pub struct Plan {
    pub local_head: CommitHash,
    pub db_head: CommitHash,
    pub merge_base: CommitHash,
    pub rollbacks: Vec<RollbackStep>,
    pub migrations: Vec<MigrationStep>,
}

impl Plan {
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

    pub fn from_db_and_local<L: LocalRepo>(db: &mut DbRepo, local: &mut L) -> Result<Self, PlanError> {
        let db_head = db.head();
        let local_head = local.head();

        let (merge_base, rollbacks) = if local.commit(&db_head).is_some() {
            (db_head.clone(), vec![])
        } else {
            let mut rollbacks = vec![];
            let mut head = db_head.clone();

            while local.commit(&head).is_none() {
                if head.is_zero() {
                    break;
                }

                let commit = db.commit(&head).ok_or_else(|| PlanError::MissingCommit(head))?;
                head = commit.parent();

                let hash = commit.migration_hash();

                match db.rollback(&hash) {
                    Some(rollback) => {
                        rollbacks.push(RollbackStep {
                            target: hash,
                            source: RollbackSource::Database,
                            text: rollback,
                            parent: commit.parent(),
                        });
                    }
                    None => {
                        match local.rollback(&hash) {
                            Some(rollback) => {
                                rollbacks.push(RollbackStep {
                                    target: hash,
                                    source: RollbackSource::Local,
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
            let commit = local.commit(&head).ok_or_else(|| PlanError::MissingCommit(head))?;
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
    MissingCommit(CommitHash),
    MissingRollback(MigrationHash),
}

#[derive(Debug, Clone)]
pub struct RollbackStep {
    pub target: MigrationHash,
    pub source: RollbackSource,
    pub text: String,
    pub parent: CommitHash,
}

impl RollbackStep {
    pub fn commit(&self) -> CommitHash {
        CommitHash::from_parent_and_hash(&self.parent, &self.target)
    }
}

#[derive(Debug, Clone)]
pub enum RollbackSource {
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
