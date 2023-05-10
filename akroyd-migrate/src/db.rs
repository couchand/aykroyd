//! Database access and migrations in the database.

use crate::hash::{CommitHash, MigrationHash};
use crate::local::LocalRepo;
use crate::plan::{MigrationStep, Plan, RollbackStep};
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

pub struct DatabaseRepo<'a> {
    txn: akroyd::sync_client::Transaction<'a>,
    head: Option<CommitHash>,
    migrations: Option<Vec<DatabaseMigration>>,
}

impl<'a> DatabaseRepo<'a> {
    /// Construct a new DatabaseRepo wrapping the provided client.
    pub fn new(client: &'a mut akroyd::sync_client::Client) -> Result<Self, tokio_postgres::Error> {
        let mut txn = client.transaction()?;

        txn.execute(&CreateTableMigrations)?;

        let head = None;
        let migrations = None;
        Ok(DatabaseRepo { txn, head, migrations })
    }

    /// Fast-forward the database to the given LocalRepo, if possible.
    pub fn fast_forward_to(mut self, local_repo: &mut LocalRepo) -> Result<(), Error> {
        let plan = Plan::from_db_and_local(&mut self, local_repo)?;

        if !plan.is_empty() && plan.is_fast_forward() {
            self.apply(&plan)?;
        }

        Ok(())
    }

    /// Apply the given plan to the database.
    pub fn apply(mut self, plan: &Plan) -> Result<(), tokio_postgres::Error> {
        assert!(self.head() == plan.db_head);

        for rollback in &plan.rollbacks {
            self.apply_rollback(rollback)?;
        }

        for migration in &plan.migrations {
            self.apply_migration(migration)?;
        }

        self.txn.commit()
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

impl<'a> std::fmt::Debug for DatabaseRepo<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "DatabaseRepo")
    }
}

pub fn fast_forward_migrate(client: &mut akroyd::sync_client::Client, mut local_repo: LocalRepo) -> Result<(), Error> {
    DatabaseRepo::new(client)?.fast_forward_to(&mut local_repo)
}

impl<'a> Repo for DatabaseRepo<'a> {
    type Commit = DatabaseMigration;
    fn head(&mut self) -> CommitHash {
        if let Some(commit) = self.head.as_ref() {
            return commit.clone();
        }

        if self.migrations.is_none() {
            let migrations = self.txn.query(&AllMigrations).unwrap(); // TODO

            self.migrations = Some(migrations);
        }

        let migrations = self.migrations.as_ref().unwrap();

        let head = if migrations.is_empty() {
            CommitHash::default()
        } else {
            let mut commits = migrations.iter().map(|m| &m.commit).collect::<Vec<_>>();
            for migration in migrations {
                if let Some(parent) = migration.parent.as_ref() {
                    commits = commits.into_iter().filter(|c| *c != parent).collect();
                }
            }
            assert_eq!(commits.len(), 1);
            commits[0].clone()
        };
        self.head = Some(head.clone());
        return head;
    }

    fn commit(&mut self, commit: &CommitHash) -> Option<Self::Commit> {
        if self.migrations.is_none() {
            let migrations = self.txn.query(&AllMigrations).unwrap(); // TODO

            self.migrations = Some(migrations);
        }

        self.migrations
            .as_ref()
            .unwrap()
            .iter()
            .find(|c| c.commit == *commit)
            .cloned()
    }

    fn rollback(&mut self, hash: &MigrationHash) -> Option<String> {
        if self.migrations.is_none() {
            let migrations = self.txn.query(&AllMigrations).unwrap(); // TODO

            self.migrations = Some(migrations);
        }

        self.migrations
            .as_ref()
            .unwrap()
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
