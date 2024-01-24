#![allow(clippy::needless_doctest_main)]
//! Aykroyd MySQL support.

pub use aykroyd;
pub use mysql;
pub use r2d2;

use aykroyd::mysql::{Client, Error};
use r2d2::ManageConnection;

/// An `r2d2::ManageConnection` for `aykroyd::mysql::Client`s.
///
/// ## Example
///
/// ```no_run
/// use std::thread;
/// use r2d2_aykroyd::mysql::AykroydConnectionManager;
/// use aykroyd::Statement;
///
/// #[derive(Statement)]
/// #[aykroyd(text = "INSERT INTO foo(bar) VALUES (?)")]
/// struct InsertFoo(i32);
///
/// fn main() {
///     let opts = mysql::Opts::from_url(
///         "mysql://user:password@locahost:3307/db_name",
///     ).unwrap();
///     let builder = mysql::OptsBuilder::from_opts(opts);
///     let manager = AykroydConnectionManager::new(builder);
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
    inner: r2d2_mysql::MySqlConnectionManager,
}

impl AykroydConnectionManager {
    /// Creates a new `AykroydConnectionManager`.
    pub fn new(params: mysql::OptsBuilder) -> AykroydConnectionManager {
        let inner = r2d2_mysql::MySqlConnectionManager::new(params);
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
