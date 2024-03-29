//! Aykroyd support for the `r2d2` connection pool.
#![warn(missing_docs)]

pub use aykroyd;
pub use postgres;
pub use r2d2;

use postgres::tls::{MakeTlsConnect, TlsConnect};
use postgres::{Config, Error, Socket};
use aykroyd::sync_client::Client;
use r2d2::ManageConnection;

/// An `r2d2::ManageConnection` for `aykroyd::Client`s.
///
/// ## Example
///
/// ```no_run
/// use std::thread;
/// use postgres::NoTls;
/// use r2d2_aykroyd::AykroydConnectionManager;
///
/// fn main() {
///     let manager = AykroydConnectionManager::new(
///         "host=localhost user=postgres".parse().unwrap(),
///         NoTls,
///     );
///     let pool = r2d2::Pool::new(manager).unwrap();
///
///     for i in 0..10i32 {
///         let pool = pool.clone();
///         thread::spawn(move || {
///             let mut client = pool.get().unwrap();
///             client.execute("INSERT INTO foo (bar) VALUES ($1)", &[&i]).unwrap();
///         });
///     }
/// }
/// ```
pub struct AykroydConnectionManager<Tls> {
    inner: r2d2_postgres::PostgresConnectionManager<Tls>,
}

impl<T> AykroydConnectionManager<T>
where
    T: MakeTlsConnect<Socket> + Clone + 'static + Sync + Send,
    T::TlsConnect: Send,
    T::Stream: Send,
    <T::TlsConnect as TlsConnect<Socket>>::Future: Send,
{
    /// Creates a new `AykroydConnectionManager`.
    pub fn new(config: Config, tls_connector: T) -> AykroydConnectionManager<T> {
        let inner = r2d2_postgres::PostgresConnectionManager::new(config, tls_connector);
        AykroydConnectionManager { inner }
    }
}

impl<T> ManageConnection for AykroydConnectionManager<T>
where
    T: MakeTlsConnect<Socket> + Clone + 'static + Sync + Send,
    T::TlsConnect: Send,
    T::Stream: Send,
    <T::TlsConnect as TlsConnect<Socket>>::Future: Send,
{
    type Connection = Client;
    type Error = Error;

    fn connect(&self) -> Result<Client, Error> {
        let client = self.inner.connect()?;
        Ok(Client::new(client))
    }

    fn is_valid(&self, client: &mut Client) -> Result<(), Error> {
        self.inner.is_valid(client.as_mut())
    }

    fn has_broken(&self, client: &mut Client) -> bool {
        self.inner.has_broken(client.as_mut())
    }
}
