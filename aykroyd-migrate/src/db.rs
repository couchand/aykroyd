//! Database access and migrations in the database.

use crate::hash::{CommitHash, MigrationHash};
use crate::local::LocalRepo;
use crate::plan::Plan;
#[cfg(any(feature = "sync", feature = "async"))]
use crate::plan::{MigrationStep, RollbackStep};
use crate::traits::{Commit, Repo};
#[cfg(feature = "sync")]
use crate::traits::Apply;
#[cfg(feature = "async")]
use crate::traits::AsyncApply;
use crate::Error;

use aykroyd::*;
use chrono::{DateTime, Utc};

#[cfg(feature = "sync")]
pub type SyncRepo<'a> = DbRepo<sync_client::Transaction<'a>>;
#[cfg(feature = "async")]
pub type AsyncRepo<'a> = DbRepo<async_client::Transaction<'a>>;

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
pub struct DbMigration {
    pub commit: CommitHash,
    pub parent: Option<CommitHash>,
    pub hash: MigrationHash,
    pub name: String,
    pub text: String,
    pub rollback: Option<String>,
    pub created_on: DateTime<Utc>,
}

#[derive(Query)]
#[query(
    row(DbMigration),
    text = "SELECT commit, parent, hash, name, text, rollback, created_on FROM migrations"
)]
pub struct AllMigrations;

#[derive(Statement)]
#[query(
    text = "INSERT INTO migrations (commit, parent, hash, name, text, rollback, created_on) VALUES ($1, $2, $3, $4, $5, $6, $7)"
)]
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
pub struct DbRepo<Txn> {
    txn: Txn,
    head: CommitHash,
    migrations: Vec<DbMigration>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeStatus {
    NothingToDo,
    Done,
}

impl<Txn> DbRepo<Txn> {
    pub fn new(txn: Txn, migrations: Vec<DbMigration>) -> Result<Self, Error> {
        let head = if migrations.is_empty() {
            CommitHash::default()
        } else {
            let mut commits = migrations.iter().map(|m| &m.commit).collect::<Vec<_>>();
            for migration in &migrations {
                if let Some(parent) = migration.parent.as_ref() {
                    commits.retain(|c| *c != parent);
                }
            }
            if commits.len() != 1 {
                let commits = commits
                    .into_iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>();
                return Err(Error::multiple_heads(&commits.join(", ")));
            }
            commits[0].clone()
        };

        Ok(DbRepo {
            txn,
            head,
            migrations,
        })
    }
}

impl<Txn> DbRepo<Txn> {
    pub fn fast_forward_plan(&self, local_repo: &LocalRepo) -> Result<Plan, Error> {
        let plan = Plan::from_db_and_local(self, local_repo)?;

        if !plan.is_fast_forward() {
            return Err(Error::divergence(&format!(
                "refusing to run {} rollbacks",
                plan.rollbacks.len()
            )));
        }

        Ok(plan)
    }
}

#[cfg(feature = "sync")]
impl<'a> DbRepo<sync_client::Transaction<'a>> {
    /// Construct a new DbRepo wrapping the provided client.
    pub fn from_client(client: &'a mut sync_client::Client) -> Result<Self, Error> {
        let mut txn = client.transaction()?;

        txn.execute(&CreateTableMigrations)?;
        let migrations = txn.query(&AllMigrations)?;

        Self::new(txn, migrations)
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

    pub fn fast_forward_migrate(
        client: &'a mut sync_client::Client,
        mut local_repo: LocalRepo,
    ) -> Result<MergeStatus, Error> {
        Self::from_client(client)?.fast_forward_to(&mut local_repo)
    }
}

#[cfg(feature = "sync")]
impl<'a> Apply for DbRepo<sync_client::Transaction<'a>> {
    type Error = tokio_postgres::Error;

    fn apply_rollback(&mut self, step: &RollbackStep) -> Result<(), tokio_postgres::Error> {
        // TODO: configurable logging
        println!("Rolling back {}...", step.target);

        self.txn.as_mut().batch_execute(&step.text)?; // TODO: the errors from this should be handled differently

        self.txn.execute(&DeleteMigration {
            commit: &step.commit(),
        })?;

        Ok(())
    }

    fn apply_migration(&mut self, step: &MigrationStep) -> Result<(), tokio_postgres::Error> {
        // TODO: configurable logging
        println!("Applying {}...", step.name);

        self.txn.as_mut().batch_execute(&step.text)?; // TODO: the errors from this should be handled differently

        self.txn.execute(&InsertMigration {
            commit: &step.commit(),
            parent: if step.parent.is_zero() {
                None
            } else {
                Some(&step.parent)
            },
            hash: &step.hash(),
            name: &step.name,
            text: &step.text,
            rollback: step.rollback.as_ref().map(AsRef::as_ref),
            created_on: Utc::now(),
        })?;

        Ok(())
    }

    fn commit(self) -> Result<(), tokio_postgres::Error> {
        self.txn.commit()
    }
}

#[cfg(feature = "async")]
impl<'a> DbRepo<async_client::Transaction<'a>> {
    /// Construct a new DbRepo wrapping the provided client.
    pub async fn from_client(
        client: &'a mut async_client::Client,
    ) -> Result<DbRepo<async_client::Transaction<'a>>, Error> {
        let mut txn = client.transaction().await?;

        txn.execute(&CreateTableMigrations).await?;
        let migrations = txn.query(&AllMigrations).await?;

        Self::new(txn, migrations)
    }

    /// Fast-forward the database to the given LocalRepo, if possible.
    pub async fn fast_forward_to(self, local_repo: &mut LocalRepo) -> Result<MergeStatus, Error> {
        let plan = self.fast_forward_plan(local_repo)?;

        if plan.is_empty() {
            return Ok(MergeStatus::NothingToDo);
        }

        self.apply(&plan).await?;
        Ok(MergeStatus::Done)
    }

    pub async fn fast_forward_migrate(
        client: &'a mut async_client::Client,
        mut local_repo: LocalRepo,
    ) -> Result<MergeStatus, Error> {
        Self::from_client(client)
            .await?
            .fast_forward_to(&mut local_repo)
            .await
    }
}

#[cfg(feature = "async")]
#[async_trait::async_trait]
impl<'a> AsyncApply for DbRepo<async_client::Transaction<'a>> {
    type Error = tokio_postgres::Error;

    async fn apply_rollback(&mut self, step: &RollbackStep) -> Result<(), tokio_postgres::Error> {
        // TODO: configurable logging
        println!("Rolling back {}...", step.target);

        self.txn.as_mut().batch_execute(&step.text).await?; // TODO: the errors from this should be handled differently

        self.txn
            .execute(&DeleteMigration {
                commit: &step.commit(),
            })
            .await?;

        Ok(())
    }

    async fn apply_migration(&mut self, step: &MigrationStep) -> Result<(), tokio_postgres::Error> {
        // TODO: configurable logging
        println!("Applying {}...", step.name);

        self.txn.as_mut().batch_execute(&step.text).await?; // TODO: the errors from this should be handled differently

        self.txn
            .execute(&InsertMigration {
                commit: &step.commit(),
                parent: if step.parent.is_zero() {
                    None
                } else {
                    Some(&step.parent)
                },
                hash: &step.hash(),
                name: &step.name,
                text: &step.text,
                rollback: step.rollback.as_ref().map(AsRef::as_ref),
                created_on: Utc::now(),
            })
            .await?;

        Ok(())
    }

    async fn commit(self) -> Result<(), tokio_postgres::Error> {
        self.txn.commit().await
    }
}

impl<Txn> std::fmt::Debug for DbRepo<Txn> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "DbRepo")
    }
}

impl<Txn> Repo for DbRepo<Txn> {
    type Commit = DbMigration;
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

impl Commit for DbMigration {
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
