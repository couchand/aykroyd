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

    #[cfg(feature = "sync")]
    pub fn from_sync_client(client: &mut akroyd::sync_client::Client) -> Result<DatabaseRepo, tokio_postgres::Error> {
        SyncBuilder::new(client)
            .build()
            .map_err(|e| match e {
                BuilderError::Database(err) => err,
                BuilderError::Protocol(_) => panic!("protocol error"),
            })
    }

    #[cfg(feature = "async")]
    pub async fn from_async_client(client: &mut akroyd::async_client::Client) -> Result<DatabaseRepo, tokio_postgres::Error> {
        AsyncBuilder::new(client)
            .build()
            .await
            .map_err(|e| match e {
                BuilderError::Database(err) => err,
                BuilderError::Protocol(_) => panic!("protocol error"),
            })
    }
}

pub enum Output<'a> {
    QueryOptIsInsertable(IsInsertable<'a>),
    QueryOptHasEnum(HasEnum<'a>),
    ExecuteCreateTableMigrationText(CreateTableMigrationText),
    ExecuteCreateTableMigrationCommit(CreateTableMigrationCommit),
    ExecuteCreateEnumMigrationDir(CreateEnumMigrationDir),
    ExecuteCreateTableMigrationCurrent(CreateTableMigrationCurrent),
    ExecuteSetCurrentMigration(SetCurrentMigration<'a>),
    QueryAllMigrations(AllMigrations),
    QueryAllCurrent(AllCurrent),
    Done(DatabaseRepo),
}

pub enum Input {
    None,
    ResultIsInsertable(Option<(String, bool)>),
    ResultHasEnum(Option<(u32, String)>),
    ResultAllMigrations(Vec<DatabaseMigration>),
    ResultAllCurrent(Vec<CurrentMigration>),
}

enum State {
    Init,
    AwaitingResultIsInsertableMigrationText,
    AwaitingResultCreateTableMigrationText,
    AwaitingResultIsInsertableMigrationCommit,
    AwaitingResultCreateTableMigrationCommit,
    AwaitingResultHasEnumMigrationDir,
    AwaitingResultCreateEnumMigrationDir,
    AwaitingResultIsInsertableMigrationCurrent,
    AwaitingResultCreateTableMigrationCurrent,
    AwaitingResultAllMigrations,
    AwaitingResultAllCurrent(Vec<DatabaseMigration>),
    Done,
    Error,
}

pub struct InvalidState(Input);

pub struct Builder(State);

impl Builder {
    pub fn new() -> Self {
        Builder(State::Init)
    }

    pub fn step(&mut self, input: Input) -> Result<Output, InvalidState> {
        let mut old_state = State::Error;
        std::mem::swap(&mut old_state, &mut self.0);
        let (new_state, output) = match (old_state, input) {
            (State::Init, Input::None) => (
                State::AwaitingResultIsInsertableMigrationText,
                Output::QueryOptIsInsertable(IsInsertable { table_name: "migration_text" })
            ),
            (State::AwaitingResultIsInsertableMigrationText, Input::ResultIsInsertable(None)) => (
                State::AwaitingResultCreateTableMigrationText,
                Output::ExecuteCreateTableMigrationText(CreateTableMigrationText),
            ),
            (State::AwaitingResultIsInsertableMigrationText, Input::ResultIsInsertable(Some((_, true)))) |
                (State::AwaitingResultCreateTableMigrationText, Input::None) => (
                State::AwaitingResultIsInsertableMigrationCommit,
                Output::QueryOptIsInsertable(IsInsertable { table_name: "migration_commit" }),
            ),
            (State::AwaitingResultIsInsertableMigrationCommit, Input::ResultIsInsertable(None)) => (
                State::AwaitingResultCreateTableMigrationCommit,
                Output::ExecuteCreateTableMigrationCommit(CreateTableMigrationCommit),
            ),
            (State::AwaitingResultIsInsertableMigrationCommit, Input::ResultIsInsertable(Some((_, true)))) |
                (State::AwaitingResultCreateTableMigrationCommit, Input::None) => (
                State::AwaitingResultHasEnumMigrationDir,
                Output::QueryOptHasEnum(HasEnum { name: "migration_dir" }),
            ),
            (State::AwaitingResultHasEnumMigrationDir, Input::ResultHasEnum(None)) => (
                State::AwaitingResultCreateEnumMigrationDir,
                Output::ExecuteCreateEnumMigrationDir(CreateEnumMigrationDir),
            ),
            (State::AwaitingResultHasEnumMigrationDir, Input::ResultHasEnum(Some(_))) |
                (State::AwaitingResultCreateEnumMigrationDir, Input::None) => (
                State::AwaitingResultIsInsertableMigrationCurrent,
                Output::QueryOptIsInsertable(IsInsertable { table_name: "migration_current" }),
            ),
            (State::AwaitingResultIsInsertableMigrationCurrent, Input::ResultIsInsertable(None)) => (
                State::AwaitingResultCreateTableMigrationCurrent,
                Output::ExecuteCreateTableMigrationCurrent(CreateTableMigrationCurrent),
            ),
            (State::AwaitingResultIsInsertableMigrationCurrent, Input::ResultIsInsertable(Some((_, true)))) |
                (State::AwaitingResultCreateTableMigrationCurrent, Input::None) => (
                State::AwaitingResultAllMigrations,
                Output::QueryAllMigrations(AllMigrations),
            ),
            (State::AwaitingResultAllMigrations, Input::ResultAllMigrations(migrations)) => (
                State::AwaitingResultAllCurrent(migrations),
                Output::QueryAllCurrent(AllCurrent),
            ),
            (State::AwaitingResultAllCurrent(migrations), Input::ResultAllCurrent(current)) => (
                State::Done,
                Output::Done(DatabaseRepo::new(current, migrations)),
            ),
            (_, input) => return Err(InvalidState(input)),
        };

        self.0 = new_state;

        Ok(output)
    }
}

// TODO: this error type isn't great
pub enum BuilderError {
    Database(tokio_postgres::Error),
    Protocol(InvalidState),
}

impl From<tokio_postgres::Error> for BuilderError {
    fn from(err: tokio_postgres::Error) -> Self {
        BuilderError::Database(err)
    }
}

impl From<InvalidState> for BuilderError {
    fn from(err: InvalidState) -> Self {
        BuilderError::Protocol(err)
    }
}

#[cfg(feature = "sync")]
struct SyncBuilder<'a>(&'a mut akroyd::sync_client::Client);

#[cfg(feature = "sync")]
impl<'a> SyncBuilder<'a> {
    fn new(client: &'a mut akroyd::sync_client::Client) -> Self {
        SyncBuilder(client)
    }

    fn step(&mut self, input: &Output) -> Result<Input, tokio_postgres::Error> {
        match input {
            Output::QueryOptIsInsertable(query) => self.0.query_opt(query).map(Input::ResultIsInsertable),
            Output::QueryOptHasEnum(query) => self.0.query_opt(query).map(Input::ResultHasEnum),
            Output::ExecuteCreateTableMigrationText(stmt) => self.0.execute(stmt).map(|_| Input::None),
            Output::ExecuteCreateTableMigrationCommit(stmt) => self.0.execute(stmt).map(|_| Input::None),
            Output::ExecuteCreateEnumMigrationDir(stmt) => self.0.execute(stmt).map(|_| Input::None),
            Output::ExecuteCreateTableMigrationCurrent(stmt) => self.0.execute(stmt).map(|_| Input::None),
            Output::ExecuteSetCurrentMigration(stmt) => self.0.execute(stmt).map(|_| Input::None),
            Output::QueryAllMigrations(query) => self.0.query(query).map(Input::ResultAllMigrations),
            Output::QueryAllCurrent(query) => self.0.query(query).map(Input::ResultAllCurrent),
            Output::Done(_) => panic!("We should have caught the Done signal!"),
        }
    }

    fn build(mut self) -> Result<DatabaseRepo, BuilderError> {
        let mut builder = Builder::new();
        let mut input = Input::None;

        loop {
            let output = builder.step(input)?;
            if let Output::Done(repo) = output {
                break Ok(repo);
            }
            input = self.step(&output)?;
        }
    }
}

#[cfg(feature = "async")]
struct AsyncBuilder<'a>(&'a mut akroyd::async_client::Client);

#[cfg(feature = "async")]
impl<'a> AsyncBuilder<'a> {
    fn new(client: &'a mut akroyd::async_client::Client) -> Self {
        AsyncBuilder(client)
    }

    async fn step(&mut self, input: &Output<'_>) -> Result<Input, tokio_postgres::Error> {
        match input {
            Output::QueryOptIsInsertable(query) => self.0.query_opt(query).await.map(Input::ResultIsInsertable),
            Output::QueryOptHasEnum(query) => self.0.query_opt(query).await.map(Input::ResultHasEnum),
            Output::ExecuteCreateTableMigrationText(stmt) => self.0.execute(stmt).await.map(|_| Input::None),
            Output::ExecuteCreateTableMigrationCommit(stmt) => self.0.execute(stmt).await.map(|_| Input::None),
            Output::ExecuteCreateEnumMigrationDir(stmt) => self.0.execute(stmt).await.map(|_| Input::None),
            Output::ExecuteCreateTableMigrationCurrent(stmt) => self.0.execute(stmt).await.map(|_| Input::None),
            Output::ExecuteSetCurrentMigration(stmt) => self.0.execute(stmt).await.map(|_| Input::None),
            Output::QueryAllMigrations(query) => self.0.query(query).await.map(Input::ResultAllMigrations),
            Output::QueryAllCurrent(query) => self.0.query(query).await.map(Input::ResultAllCurrent),
            Output::Done(_) => panic!("We should have caught the Done signal!"),
        }
    }

    async fn build(mut self) -> Result<DatabaseRepo, BuilderError> {
        let mut builder = Builder::new();
        let mut input = Input::None;

        loop {
            let output = builder.step(input)?;
            if let Output::Done(repo) = output {
                break Ok(repo);
            }
            input = self.step(&output).await?;
        }
    }
}
