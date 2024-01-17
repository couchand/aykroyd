//! Aykroyd PostgreSQL support.

pub use deadpool;
pub use aykroyd;
pub use tokio_postgres;

use async_trait::async_trait;
use aykroyd::tokio_postgres::Client;
use tokio_postgres::tls::{MakeTlsConnect, TlsConnect};
use tokio_postgres::Socket;

type RecycleResult = deadpool::managed::RecycleResult<tokio_postgres::Error>;
type RecycleError = deadpool::managed::RecycleError<tokio_postgres::Error>;

pub use deadpool_postgres::{ManagerConfig, RecyclingMethod};

pub type Object<T> = deadpool::managed::Object<Manager<T>>;
pub type Pool<T> = deadpool::managed::Pool<Manager<T>, deadpool::managed::Object<Manager<T>>>;
pub type PoolBuilder<T> = deadpool::managed::PoolBuilder<Manager<T>>;
pub type PoolError = deadpool::managed::PoolError<tokio_postgres::Error>;

#[derive(Debug)]
pub struct Manager<T> {
    config: ManagerConfig,
    pg_config: tokio_postgres::Config,
    tls: T,
}

impl<T> Manager<T> {
    pub fn new(pg_config: tokio_postgres::Config, tls: T) -> Self {
        Self::from_config(pg_config, tls, ManagerConfig::default())
    }

    pub fn from_config(pg_config: tokio_postgres::Config, tls: T, config: ManagerConfig) -> Self {
        Manager { config, pg_config, tls }
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
