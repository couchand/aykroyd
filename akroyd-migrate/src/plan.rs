use crate::hash2::{CommitHash, MigrationHash};
use crate::traits::{Repo, Commit};

#[derive(Debug)]
pub struct DbRepo;

impl Repo for DbRepo {
    type Commit = DbCommit;

    fn head(&self) -> CommitHash {
        "1234567890123456789012345678901234567890123456789012345678900000".parse().unwrap()
    }

    fn commit(&self, commit: &CommitHash) -> Option<DbCommit> {
        let hash1 = "1234567890123456789012345678901234567890123456789012345678900000".parse().unwrap();
        if *commit == hash1 {
            Some(DbCommit)
        } else {
            None
        }
    }

    fn rollback(&self, hash: &MigrationHash) -> Option<String> {
        let hash1 = "1234567890123456789012345678901234567890123456789012345678901234".parse().unwrap();
        if *hash == hash1 {
            Some("DROP TABLE emails".into())
        } else {
            None
        }
    }
}

impl DbRepo {
    fn apply(&mut self, plan: &Plan) {
        plan.verify().unwrap();

        todo!()
    }
}

#[derive(Debug)]
pub struct DbCommit;

impl Commit for DbCommit {
    fn commit_hash(&self) -> CommitHash {
        todo!()
    }

    fn parent(&self) -> CommitHash {
        "45f27016543e7ecf16eb52ab920f7ba91f87bf2863c39578f10b6c8722cebffa".parse().unwrap()
    }

    fn migration_name(&self) -> String {
        todo!()
    }

    fn migration_text(&self) -> String {
        todo!()
    }

    fn migration_hash(&self) -> MigrationHash {
        "1234567890123456789012345678901234567890123456789012345678901234".parse().unwrap()
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

    pub fn from_db_and_local<Local: Repo>(db: &mut DbRepo, local: &mut Local) -> Result<Self, PlanError> {
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
