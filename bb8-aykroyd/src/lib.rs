//! Postgres support for the `bb8` connection pool.
#![deny(missing_docs, missing_debug_implementations)]

pub use bb8;
pub use aykroyd;

use aykroyd::async_client::Client;
use async_trait::async_trait;
use tokio_postgres::config::Config;
use tokio_postgres::tls::{MakeTlsConnect, TlsConnect};
use tokio_postgres::{Error, Socket};

use std::fmt;

/// A `bb8::ManageConnection` for `aykroyd::Connection`s.
#[derive(Clone)]
pub struct AykroydConnectionManager<Tls>
where
    Tls: MakeTlsConnect<Socket>,
{
    inner: bb8_postgres::PostgresConnectionManager<Tls>,
}

impl<Tls> AykroydConnectionManager<Tls>
where
    Tls: MakeTlsConnect<Socket>,
{
    /// Create a new `AykroydConnectionManager` with the specified `config`.
    pub fn new(config: Config, tls: Tls) -> AykroydConnectionManager<Tls> {
        let inner = bb8_postgres::PostgresConnectionManager::new(config, tls);
        AykroydConnectionManager { inner }
    }

    /// Create a new `AykroydConnectionManager`, parsing the config from `params`.
    pub fn new_from_stringlike<T>(
        params: T,
        tls: Tls,
    ) -> Result<AykroydConnectionManager<Tls>, Error>
    where
        T: ToString,
    {
        let inner = bb8_postgres::PostgresConnectionManager::new_from_stringlike(params, tls)?;
        Ok(AykroydConnectionManager { inner })
    }
}

#[async_trait]
impl<Tls> bb8::ManageConnection for AykroydConnectionManager<Tls>
where
    Tls: MakeTlsConnect<Socket> + Clone + Send + Sync + 'static,
    <Tls as MakeTlsConnect<Socket>>::Stream: Send + Sync,
    <Tls as MakeTlsConnect<Socket>>::TlsConnect: Send,
    <<Tls as MakeTlsConnect<Socket>>::TlsConnect as TlsConnect<Socket>>::Future: Send,
{
    type Connection = Client;
    type Error = Error;

    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        let client = self.inner.connect().await?;
        Ok(Client::new(client))
    }

    async fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        self.inner.is_valid(conn.as_mut()).await
    }

    fn has_broken(&self, conn: &mut Self::Connection) -> bool {
        self.inner.has_broken(conn.as_mut())
    }
}

impl<Tls> fmt::Debug for AykroydConnectionManager<Tls>
where
    Tls: MakeTlsConnect<Socket>,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("AykroydConnectionManager")
            .field("inner", &self.inner)
            .finish()
    }
}
