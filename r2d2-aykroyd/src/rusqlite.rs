#![allow(clippy::needless_doctest_main)]
//! Aykroyd SQLite support.

pub use aykroyd;
pub use rusqlite;
pub use r2d2;

use aykroyd::rusqlite::{Client, Error};
use r2d2::ManageConnection;

/// An `r2d2::ManageConnection` for `aykroyd::rusqlite::Client`s.
///
/// ## Example
///
/// ```no_run
/// use std::thread;
/// use r2d2_aykroyd::rusqlite::AykroydConnectionManager;
/// use aykroyd::Statement;
///
/// #[derive(Statement)]
/// #[aykroyd(text = "INSERT INTO foo(bar) VALUES ($1)")]
/// struct InsertFoo(i32);
///
/// fn main() {
///     let manager = AykroydConnectionManager::file("file.db");
///     let pool = r2d2::Pool::new(manager).unwrap();
///
///     for i in 0..10i32 {
///         let pool = pool.clone();
///         thread::spawn(move || {
///             let mut client = pool.get().unwrap();
///             client.execute(&InsertFoo(i)).unwrap();
///         });
///     }
/// }
/// ```
#[derive(Debug)]
pub struct AykroydConnectionManager {
    inner: r2d2_sqlite::SqliteConnectionManager,
}

impl AykroydConnectionManager {
    /// Creates a new SqliteConnectionManager from file.
    ///
    /// See rusqlite::Connection::open
    pub fn file<P: AsRef<std::path::Path>>(path: P) -> Self {
        let inner = r2d2_sqlite::SqliteConnectionManager::file(path);
        AykroydConnectionManager { inner }
    }

    /// Creates a new SqliteConnectionManager from memory.
    pub fn memory() -> Self {
        let inner = r2d2_sqlite::SqliteConnectionManager::memory();
        AykroydConnectionManager { inner }
    }

    /// Converts `AykroydConnectionManager` into one that sets
    /// `OpenFlags` upon connection creation.
    ///
    /// See [`rusqlite::OpenFlags`] for a list of available flags.
    pub fn with_flags(self, flags: rusqlite::OpenFlags) -> Self {
        let AykroydConnectionManager { inner } = self;
        let inner = inner.with_flags(flags);
        AykroydConnectionManager { inner }
    }

    /// Converts `AykroydConnectionManager` into one that calls
    /// an initialization function upon connection creation.
    /// Could be used to set PRAGMAs, for example.
    ///
    /// ### Example
    ///
    /// Make a `AykroydConnectionManager` that sets the foreign_keys
    /// pragma to true for every connection.
    ///
    /// ```rust,no_run
    /// # use r2d2_aykroyd::rusqlite::{AykroydConnectionManager};
    /// let manager = AykroydConnectionManager::file("app.db")
    ///     .with_init(|c| c.execute_batch("PRAGMA foreign_keys=1;"));
    /// ```
    pub fn with_init<F>(self, init: F) -> Self
    where
        F: Fn(
            &mut rusqlite::Connection
        ) -> Result<(), rusqlite::Error> + Send + Sync + 'static
    {
        let AykroydConnectionManager { inner } = self;
        let inner = inner.with_init(init);
        AykroydConnectionManager { inner }
    }
}

impl ManageConnection for AykroydConnectionManager {
    type Connection = Client;
    type Error = Error;

    fn connect(&self) -> Result<Client, Error> {
        let client = self.inner.connect().map_err(Error::connect)?;
        Ok(Client::from(client))
    }

    fn is_valid(&self, client: &mut Client) -> Result<(), Error> {
        self.inner.is_valid(client.as_mut()).map_err(Error::connect)
    }

    fn has_broken(&self, client: &mut Client) -> bool {
        self.inner.has_broken(client.as_mut())
    }
}
