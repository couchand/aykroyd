//! Database access and migrations in the database.

use crate::hash::{CommitHash, MigrationHash};
use crate::local::LocalRepo;
use crate::plan::Plan;
#[cfg(any(feature = "sync", feature = "async"))]
use crate::plan::{MigrationStep, RollbackStep};
use crate::traits::{Commit, Repo};
use crate::Error;

use akroyd::*;
use chrono::{DateTime, Utc};

#[derive(Statement)]
#[query(text = "
CREATE TABLE IF NOT EXISTS migrations (
    commit BYTEA PRIMARY KEY,
    parent BYTEA REFERENCES migrations,
    hash BYTEA NOT NULL,
    name TEXT NOT NULL,
    text TEXT NOT NULL,
    rollback TEXT,
    created_on TIMESTAMPTZ NOT NULL
)
")]
pub struct CreateTableMigrations;

#[derive(Debug, Clone, FromRow)]
pub struct DatabaseMigration {
    pub commit: CommitHash,
    pub parent: Option<CommitHash>,
    pub hash: MigrationHash,
    pub name: String,
    pub text: String,
    pub rollback: Option<String>,
    pub created_on: DateTime<Utc>,
}

#[derive(Query)]
#[query(row(DatabaseMigration), text = "SELECT commit, parent, hash, name, text, rollback, created_on FROM migrations")]
pub struct AllMigrations;

#[derive(Statement)]
#[query(text = "INSERT INTO migrations (commit, parent, hash, name, text, rollback, created_on) VALUES ($1, $2, $3, $4, $5, $6, $7)")]
pub struct InsertMigration<'a> {
    pub commit: &'a CommitHash,
    pub parent: Option<&'a CommitHash>,
    pub hash: &'a MigrationHash,
    pub name: &'a str,
    pub text: &'a str,
    pub rollback: Option<&'a str>,
    pub created_on: DateTime<Utc>,
}

#[derive(Statement)]
#[query(text = "DELETE FROM migrations WHERE commit = $1")]
pub struct DeleteMigration<'a> {
    pub commit: &'a CommitHash,
}

#[cfg_attr(all(not(feature = "sync"), not(feature = "async")), allow(dead_code))]
pub struct DatabaseRepo<Txn> {
    txn: Txn,
    head: CommitHash,
    migrations: Vec<DatabaseMigration>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeStatus {
    NothingToDo,
    Done,
}

impl<Txn> DatabaseRepo<Txn> {
    pub fn new(txn: Txn, migrations: Vec<DatabaseMigration>) -> Result<Self, Error> {
        let head = if migrations.is_empty() {
            CommitHash::default()
        } else {
            let mut commits = migrations.iter().map(|m| &m.commit).collect::<Vec<_>>();
            for migration in &migrations {
                if let Some(parent) = migration.parent.as_ref() {
                    commits = commits.into_iter().filter(|c| *c != parent).collect();
                }
            }
            if commits.len() != 1 {
                let commits = commits.into_iter().map(ToString::to_string).collect::<Vec<_>>();
                return Err(Error::multiple_heads(&commits.join(", ")));
            }
            commits[0].clone()
        };

        Ok(DatabaseRepo { txn, head, migrations })
    }
}

impl<Txn> DatabaseRepo<Txn> where Self: Repo {
    pub fn fast_forward_plan(&self, local_repo: &LocalRepo) -> Result<Plan, Error> {
        let plan = Plan::from_db_and_local(self, local_repo)?;

        if !plan.is_fast_forward() {
            return Err(Error::divergence(&format!("refusing to run {} rollbacks", plan.rollbacks.len())));
        }

        Ok(plan)
    }
}

#[cfg(feature = "sync")]
impl<'a> DatabaseRepo<akroyd::sync_client::Transaction<'a>> {
    /// Construct a new DatabaseRepo wrapping the provided client.
    pub fn from_client(client: &'a mut akroyd::sync_client::Client) -> Result<Self, Error> {
        let mut txn = client.transaction()?;

        txn.execute(&CreateTableMigrations)?;
        let migrations = txn.query(&AllMigrations)?;

        DatabaseRepo::new(txn, migrations)
    }

    /// Fast-forward the database to the given LocalRepo, if possible.
    pub fn fast_forward_to(self, local_repo: &mut LocalRepo) -> Result<MergeStatus, Error> {
        let plan = self.fast_forward_plan(local_repo)?;

        if plan.is_empty() {
            return Ok(MergeStatus::NothingToDo);
        }

        self.apply(&plan)?;
        Ok(MergeStatus::Done)
    }

    /// Apply the given plan to the database.
    pub fn apply(mut self, plan: &Plan) -> Result<(), Error> {
        assert!(self.head() == plan.db_head);

        for rollback in &plan.rollbacks {
            self.apply_rollback(rollback)?;
        }

        for migration in &plan.migrations {
            self.apply_migration(migration)?;
        }

        self.txn.commit()?;

        Ok(())
    }

    fn apply_rollback(&mut self, step: &RollbackStep) -> Result<(), tokio_postgres::Error> {
        // TODO: configurable logging
        println!("Rolling back {}...", step.target);

        self.txn.as_mut().execute(&step.text, &[])?; // TODO: the errors from this should be handled differently

        self.txn.execute(&DeleteMigration {
            commit: &step.commit(),
        })?;

        Ok(())
    }

    fn apply_migration(&mut self, step: &MigrationStep) -> Result<(), tokio_postgres::Error> {
        // TODO: configurable logging
        println!("Applying {}...", step.name);

        self.txn.as_mut().execute(&step.text, &[])?; // TODO: the errors from this should be handled differently

        self.txn.execute(&InsertMigration {
            commit: &step.commit(),
            parent: if step.parent.is_zero() { None } else { Some(&step.parent) },
            hash: &step.hash(),
            name: &step.name,
            text: &step.text,
            rollback: step.rollback.as_ref().map(AsRef::as_ref),
            created_on: Utc::now(),
        })?;

        Ok(())
    }
}

impl<Txn> std::fmt::Debug for DatabaseRepo<Txn> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "DatabaseRepo")
    }
}

#[cfg(feature = "sync")]
pub fn fast_forward_migrate(client: &mut akroyd::sync_client::Client, mut local_repo: LocalRepo) -> Result<MergeStatus, Error> {
    DatabaseRepo::from_client(client)?.fast_forward_to(&mut local_repo)
}

#[cfg(feature = "async")]
pub async fn fast_forward_migrate_async(client: &mut akroyd::async_client::Client, mut local_repo: LocalRepo) -> Result<MergeStatus, Error> {
    DatabaseRepo::from_client(client)?.fast_forward_to(&mut local_repo).await
}

impl<Txn> Repo for DatabaseRepo<Txn> {
    type Commit = DatabaseMigration;
    fn head(&self) -> CommitHash {
        self.head.clone()
    }

    fn commit(&self, commit: &CommitHash) -> Option<Self::Commit> {
        self.migrations
            .iter()
            .find(|c| c.commit == *commit)
            .cloned()
    }

    fn rollback(&self, hash: &MigrationHash) -> Option<String> {
        self.migrations
            .iter()
            .find(|r| r.hash == *hash)
            .and_then(|r| r.rollback.clone())
    }
}

impl Commit for DatabaseMigration {
    fn commit_hash(&self) -> CommitHash {
        self.commit.clone()
    }

    fn parent(&self) -> CommitHash {
        self.parent.clone().unwrap_or_default()
    }

    fn migration_name(&self) -> String {
        self.name.clone()
    }

    fn migration_text(&self) -> String {
        self.text.clone()
    }

    fn migration_hash(&self) -> MigrationHash {
        self.hash.clone()
    }
}
