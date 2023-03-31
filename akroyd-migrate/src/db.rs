//! Database access and migrations in the database.

use akroyd::*;
use chrono::{DateTime, Utc};

#[derive(FromRow)]
pub struct DatabaseMigration {
    pub name: String,
    pub hash: String,
    pub applied_on: DateTime<Utc>,
}

#[derive(Query)]
#[query(
    text = "SELECT name, hash, applied_on FROM migrations",
    row(DatabaseMigration)
)]
pub struct GetAllMigrations;

#[derive(Statement)]
#[query(text = "INSERT INTO migrations (name, hash, applied_on) VALUES ($1, $2, $3)")]
pub struct InsertMigration<'a> {
    pub name: &'a str,
    pub hash: &'a str,
    pub applied_on: DateTime<Utc>,
}
