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

#[derive(QueryOne)]
#[query(row((u32, String)), text = "
SELECT oid, typname
FROM pg_type t
WHERE (t.typrelid = 0 or (select c.relkind = 'c' from pg_catalog.pg_class c where c.oid = t.typrelid))
AND NOT EXISTS (SELECT 1 FROM pg_catalog.pg_type el WHERE el.oid = t.typelem AND el.typarray = t.oid)
AND t.typname = $1
")]
pub struct HasEnum<'a> {
    pub name: &'a str,
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

#[derive(Statement)]
#[query(text = "CREATE TYPE migration_dir AS ENUM ('up', 'down')")]
pub struct CreateEnumMigrationDir;

#[derive(PgEnum, Debug, PartialEq, Eq, Hash, Clone, Copy)]
// TODO: attribute to rename
pub enum MigrationDir {
    Up,
    Down,
}

#[derive(Statement)]
#[query(text = "
CREATE TABLE migration_current (
    hash TEXT NOT NULL,
    dir MIGRATION_DIR UNIQUE NOT NULL
)
")]
pub struct CreateTableMigrationCurrent;

#[derive(Debug, Clone, FromRow)]
pub struct CurrentMigration {
    pub hash: MigrationHash,
    pub dir: MigrationDir,
}

#[derive(Query)]
#[query(row(CurrentMigration), text = "SELECT hash, dir FROM migration_current")]
pub struct AllCurrent;

#[derive(Statement)]
#[query(text = "INSERT INTO migration_current (hash, dir) VALUES ($2, $1) ON CONFLICT (dir) DO UPDATE SET hash = excluded.hash")]
pub struct SetCurrentMigration<'a>(pub MigrationDir, pub &'a MigrationHash);

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

#[derive(Debug)]
pub struct DatabaseRepo {
    up: MigrationHash,
    down: Option<MigrationHash>,
    commits: Vec<DatabaseMigration>,
}

impl DatabaseRepo {
    /// Construct a new DatabaseRepo from the contents of the database.
    ///
    /// Use the queries [`AllCurrent`](./struct.AllCurrent.html) and [`AllMigrations`](./struct.AllMigrations.html)
    /// to get these values.
    pub fn new(current: Vec<CurrentMigration>, commits: Vec<DatabaseMigration>) -> Self {
        let mut up = None;
        let mut down = None;

        for migration in current {
            match migration.dir {
                MigrationDir::Up => up = Some(migration.hash),
                MigrationDir::Down => down = Some(migration.hash),
            }
        }

        let up = up.unwrap_or(MigrationHash::ZERO);

        DatabaseRepo { up, down, commits }
    }
}
