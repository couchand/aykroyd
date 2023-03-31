//! Database access and migrations in the database.

use crate::hash::MigrationHash;

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
CREATE TABLE migration_text (
    hash TEXT PRIMARY KEY,
    text TEXT NOT NULL
)
")]
pub struct CreateTableMigrationText;

#[derive(Statement)]
#[query(text = "
CREATE TABLE migration_commit (
    hash TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    deps TEXT[] NOT NULL,
    text_hash TEXT NOT NULL,
    created_on TIMESTAMPTZ NOT NULL
)
")]
pub struct CreateTableMigrationCommit;

#[derive(Debug, Clone, FromRow)]
pub struct DatabaseMigration {
    pub commit_hash: MigrationHash,
    pub name: String,
    pub deps: Vec<MigrationHash>,
    pub text_hash: MigrationHash,
    pub text: Option<String>,
    pub created_on: DateTime<Utc>,
}

#[derive(Query)]
#[query(
    text = "
        SELECT migration_commit.hash AS commit_hash, name, deps, text_hash, migration_text.text AS text, created_on
        FROM migration_commit
        LEFT JOIN migration_text ON migration_commit.text_hash = migration_text.hash
    ",
    row(DatabaseMigration)
)]
pub struct AllMigrations;

#[derive(Statement)]
#[query(text = "INSERT INTO migration_text (hash, text) VALUES ($1, $2) ON CONFLICT DO NOTHING")]
pub struct InsertMigrationText<'a> {
    pub hash: &'a MigrationHash,
    pub text: &'a str,
}

#[derive(Statement)]
#[query(text = "INSERT INTO migration_commit (hash, name, deps, text_hash, created_on) VALUES ($1, $2, $3, $4, $5)")]
pub struct InsertMigrationCommit<'a> {
    pub commit_hash: &'a MigrationHash,
    pub name: &'a str,
    pub deps: &'a [MigrationHash],
    pub text_hash: &'a MigrationHash,
    pub created_on: DateTime<Utc>,
}
