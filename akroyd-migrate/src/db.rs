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
CREATE TABLE migrations (
    hash TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    text TEXT NOT NULL
)
")]
pub struct CreateTableMigrations;

#[derive(Debug, Clone, FromRow)]
pub struct DatabaseMigration {
    pub hash: MigrationHash,
    pub name: String,
    pub text: String,
}

#[derive(Query)]
#[query(
    text = "SELECT hash, name, text FROM migrations",
    row(DatabaseMigration)
)]
pub struct AllMigrations;

#[derive(Statement)]
#[query(text = "INSERT INTO migrations (hash, name, text) VALUES ($1, $2, $3)")]
pub struct InsertMigration<'a> {
    pub hash: &'a MigrationHash,
    pub name: &'a str,
    pub text: &'a str,
}
