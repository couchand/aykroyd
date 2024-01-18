//! Aykroyd PostgreSQL support.

pub use aykroyd;
pub use deadpool;
pub use tokio_postgres;

use async_trait::async_trait;
use aykroyd::tokio_postgres::Client;
use tokio_postgres::tls::{MakeTlsConnect, TlsConnect};
use tokio_postgres::Socket;

type RecycleResult = deadpool::managed::RecycleResult<tokio_postgres::Error>;
type RecycleError = deadpool::managed::RecycleError<tokio_postgres::Error>;

pub use deadpool_postgres::{ManagerConfig, RecyclingMethod};

/// An object managed by this pool, parameterized on TLS.
pub type Object<T> = deadpool::managed::Object<Manager<T>>;
/// The pool type, parameterized on TLS.
pub type Pool<T> = deadpool::managed::Pool<Manager<T>, deadpool::managed::Object<Manager<T>>>;
/// A builder for the pool type, parameterized on TLS.
pub type PoolBuilder<T> = deadpool::managed::PoolBuilder<Manager<T>>;
/// This pool's error type.
pub type PoolError = deadpool::managed::PoolError<tokio_postgres::Error>;

/// A manager for `aykroyd` database connections.
#[derive(Debug)]
pub struct Manager<T> {
    config: ManagerConfig,
    pg_config: tokio_postgres::Config,
    tls: T,
}

impl<T> Manager<T> {
    /// Create a pool manager from the given `tokio_postgres::Config`, with the default `ManagerConfig`.
    pub fn new(pg_config: tokio_postgres::Config, tls: T) -> Self {
        Self::from_config(pg_config, tls, ManagerConfig::default())
    }

    /// Create a pool manager from the given `tokio_postgres::Config` and `ManagerConfig`.
    pub fn from_config(pg_config: tokio_postgres::Config, tls: T, config: ManagerConfig) -> Self {
        Manager {
            config,
            pg_config,
            tls,
        }
    }
}

#[async_trait]
impl<T> deadpool::managed::Manager for Manager<T>
where
    T: MakeTlsConnect<Socket> + Clone + Sync + Send + 'static,
    T::Stream: Sync + Send,
    T::TlsConnect: Sync + Send,
    <T::TlsConnect as TlsConnect<Socket>>::Future: Send,
{
    type Type = Client;
    type Error = tokio_postgres::Error;

    async fn create(&self) -> Result<Client, tokio_postgres::Error> {
        let (client, connection) = self.pg_config.connect(self.tls.clone()).await?;
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                //log::warn!(target: "deadpool.postgres", "Connection error: {}", e);
                panic!("Error in deadpool-aykroyd: connection error: {e}");
            }
        });
        Ok(Client::new(client))
    }

    async fn recycle(&self, client: &mut Client) -> RecycleResult {
        if client.as_ref().is_closed() {
            //log::info!(target: "deadpool.postgres", "Connection could not be recycled: Connection closed");
            return Err(RecycleError::StaticMessage("Connection closed"));
        }
        match self.config.recycling_method.query() {
            Some(sql) => match client.as_ref().simple_query(sql).await {
                Ok(_) => Ok(()),
                Err(e) => {
                    //log::info!(target: "deadpool.postgres", "Connection could not be recycled: {}", e);
                    Err(e.into())
                }
            },
            None => Ok(()),
        }
    }
}
