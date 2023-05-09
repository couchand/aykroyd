//! Database access and migrations in the database.

use crate::hash::{CommitHash, MigrationHash};
use crate::plan::{MigrationStep, Plan, RollbackStep};
use crate::traits::{Commit, Repo};

use akroyd::*;
use chrono::{DateTime, Utc};

#[derive(QueryOne)]
#[query(row((String, bool)), text = "
SELECT table_name, is_insertable_into::BOOL
FROM information_schema.tables
WHERE table_name = $1
")]
pub struct IsInsertable<'a> {
    pub table_name: &'a str,
}

#[derive(Statement)]
#[query(text = "
CREATE TABLE migration_text2 (
    hash BYTEA PRIMARY KEY,
    name TEXT NOT NULL,
    text TEXT NOT NULL
)
")]
pub struct CreateTableMigrationText;

#[derive(Statement)]
#[query(text = "
CREATE TABLE rollback_text2 (
    hash BYTEA PRIMARY KEY,
    text TEXT NOT NULL
)
")]
pub struct CreateTableRollbackText;

#[derive(Statement)]
#[query(text = "
CREATE TABLE migration_commit2 (
    commit BYTEA PRIMARY KEY,
    parent BYTEA REFERENCES migration_commit2,
    hash BYTEA NOT NULL REFERENCES migration_text2,
    created_on TIMESTAMPTZ NOT NULL
)
")]
pub struct CreateTableMigrationCommit;

#[derive(Debug, Clone, FromRow)]
pub struct Head {
    pub commit: CommitHash,
}

#[derive(QueryOne)]
#[query(
    text = "
        SELECT c1.commit AS commit
        FROM migration_commit2 c1
        LEFT JOIN migration_commit2 c2 ON c1.commit = c2.parent
        WHERE c2.commit IS NULL
    ",
    row(Head),
)]
pub struct QueryHead;

#[derive(Debug, Clone, FromRow)]
pub struct DatabaseMigration {
    pub commit: CommitHash,
    pub parent: Option<CommitHash>,
    pub hash: MigrationHash,
    pub name: Option<String>,
    pub text: Option<String>,
    pub created_on: DateTime<Utc>,
}

#[derive(Query)]
#[query(
    text = "
        SELECT commit, parent, migration_commit2.hash AS hash, migration_text2.name AS name, migration_text2.text AS text, created_on
        FROM migration_commit2
        LEFT JOIN migration_text2 ON migration_commit2.hash = migration_text2.hash
    ",
    row(DatabaseMigration)
)]
pub struct AllMigrations;

#[derive(Debug, Clone, FromRow)]
pub struct DatabaseRollback {
    pub hash: MigrationHash,
    pub text: String,
}

#[derive(Query)]
#[query(text = "SELECT hash, text FROM rollback_text2", row(DatabaseRollback))]
pub struct AllRollbacks;

#[derive(Statement)]
#[query(text = "INSERT INTO migration_text2 (hash, name, text) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING")]
pub struct InsertMigrationText<'a> {
    pub hash: &'a MigrationHash,
    pub name: &'a str,
    pub text: &'a str,
}

#[derive(Statement)]
#[query(text = "INSERT INTO rollback_text2 (hash, text) VALUES ($1, $2) ON CONFLICT (hash) DO UPDATE SET text = excluded.text")]
pub struct InsertRollbackText<'a> {
    pub hash: &'a MigrationHash,
    pub text: &'a str,
}

#[derive(Statement)]
#[query(text = "INSERT INTO migration_commit2 (commit, parent, hash, created_on) VALUES ($1, $2, $3, $4)")]
pub struct InsertMigrationCommit<'a> {
    pub commit: &'a CommitHash,
    pub parent: Option<&'a CommitHash>,
    pub hash: &'a MigrationHash,
    pub created_on: DateTime<Utc>,
}

#[derive(Statement)]
#[query(text = "DELETE FROM migration_commit2 WHERE commit = $1")]
pub struct DeleteMigrationCommit<'a> {
    pub commit: &'a CommitHash,
}

pub struct DatabaseRepo<'a> {
    txn: akroyd::sync_client::Transaction<'a>,
    head: Option<CommitHash>,
    migrations: Option<Vec<DatabaseMigration>>,
    rollbacks: Option<Vec<DatabaseRollback>>,
}

impl<'a> DatabaseRepo<'a> {
    /// Construct a new DatabaseRepo wrapping the provided client.
    pub fn new(client: &'a mut akroyd::sync_client::Client) -> Result<Self, tokio_postgres::Error> {
        let mut txn = client.transaction()?;

        match txn.query_opt(&IsInsertable { table_name: "migration_text2" })? {
            None => {
                txn.execute(&CreateTableMigrationText)?;
            }
            Some((_, false)) => panic!("DB config issue!"),
            _ => {}
        }

        match txn.query_opt(&IsInsertable { table_name: "rollback_text2" })? {
            None => {
                txn.execute(&CreateTableRollbackText)?;
            }
            Some((_, false)) => panic!("DB config issue!"),
            _ => {}
        }

        match txn.query_opt(&IsInsertable { table_name: "migration_commit2" })? {
            None => {
                txn.execute(&CreateTableMigrationCommit)?;
            }
            Some((_, false)) => panic!("DB config issue!"),
            _ => {}
        }

        let head = None;
        let migrations = None;
        let rollbacks = None;
        Ok(DatabaseRepo { txn, head, migrations, rollbacks })
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
        self.txn.as_mut().execute(&step.text, &[])?; // TODO: the errors from this should be handled differently

        self.txn.execute(&DeleteMigrationCommit {
            commit: &step.commit(),
        })?;

        Ok(())
    }

    fn apply_migration(&mut self, step: &MigrationStep) -> Result<(), tokio_postgres::Error> {
        self.txn.as_mut().execute(&step.text, &[])?; // TODO: the errors from this should be handled differently

        self.txn.execute(&InsertMigrationText {
            hash: &step.hash(),
            name: &step.name,
            text: &step.text,
        })?;

        self.txn.execute(&InsertMigrationCommit {
            commit: &step.commit(),
            parent: if step.parent.is_zero() { None } else { Some(&step.parent) },
            hash: &step.hash(),
            created_on: Utc::now(),
        })?;

        if let Some(rollback) = step.rollback.as_ref() {
            self.txn.execute(&InsertRollbackText {
                hash: &step.hash(),
                text: rollback,
            })?;
        }

        Ok(())
    }
}

impl<'a> std::fmt::Debug for DatabaseRepo<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "DatabaseRepo")
    }
}

impl<'a> Repo for DatabaseRepo<'a> {
    type Commit = DatabaseMigration;
    fn head(&mut self) -> CommitHash {
        if let Some(commit) = self.head.as_ref() {
            return commit.clone();
        }

        if let Some(migrations) = self.migrations.as_ref() {
            let mut commits = migrations.iter().map(|m| &m.commit).collect::<Vec<_>>();
            for migration in migrations {
                if let Some(parent) = migration.parent.as_ref() {
                    commits = commits.into_iter().filter(|c| *c != parent).collect();
                }
            }
            assert_eq!(commits.len(), 1);
            self.head = Some(commits[0].clone());
            return commits[0].clone();
        }

        let head = match self.txn.query_opt(&QueryHead).unwrap() { // TODO
            Some(head) => head.commit,
            None => CommitHash::default(),
        };
        self.head = Some(head.clone());
        head
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
        if self.rollbacks.is_none() {
            let rollbacks = self.txn.query(&AllRollbacks).unwrap(); // TODO

            self.rollbacks = Some(rollbacks);
        }

        self.rollbacks
            .as_ref()
            .unwrap()
            .iter()
            .find(|r| r.hash == *hash)
            .map(|r| r.text.to_string())
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
        self.name.clone().unwrap_or_default()
    }

    fn migration_text(&self) -> String {
        self.text.clone().unwrap_or_default()
    }

    fn migration_hash(&self) -> MigrationHash {
        self.hash.clone()
    }
}
