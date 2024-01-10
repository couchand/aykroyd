//! An asynchronous, pipelined, PostgreSQL client.

use crate::client::{FromColumnIndexed, FromColumnNamed, ToParam};
use crate::query::StaticQueryText;
use crate::{Error, FromRow, Query, Statement};

/// A convenience function which parses a connection string and connects to the database.
///
/// See the documentation for tokio_postgres::Config for details on the connection string format.
pub async fn connect<T>(
    config: &str,
    tls: T,
) -> Result<
    (
        Client,
        tokio_postgres::Connection<tokio_postgres::Socket, T::Stream>,
    ),
    tokio_postgres::Error,
>
where
    T: tokio_postgres::tls::MakeTlsConnect<tokio_postgres::Socket>,
{
    let (client, connection) = tokio_postgres::connect(config, tls).await?;
    Ok((client.into(), connection))
}

impl<T> FromColumnIndexed<Client> for T
where
    T: tokio_postgres::types::FromSqlOwned,
{
    fn from_column(
        row: &tokio_postgres::Row,
        index: usize,
    ) -> Result<Self, Error<tokio_postgres::Error>> {
        row.try_get(index).map_err(Error::from_column)
    }
}

impl<T> FromColumnNamed<Client> for T
where
    T: tokio_postgres::types::FromSqlOwned,
{
    fn from_column(
        row: &tokio_postgres::Row,
        name: &str,
    ) -> Result<Self, Error<tokio_postgres::Error>> {
        row.try_get(name).map_err(Error::from_column)
    }
}

impl<T> ToParam<Client> for T
where
    T: tokio_postgres::types::ToSql + Sync,
{
    fn to_param(&self) -> &(dyn tokio_postgres::types::ToSql + Sync) {
        self
    }
}

pub struct Client {
    client: tokio_postgres::Client,
    statements: std::collections::HashMap<String, tokio_postgres::Statement>,
}

impl crate::client::Client for Client {
    type Row<'a> = tokio_postgres::Row;
    type Param<'a> = &'a (dyn tokio_postgres::types::ToSql + Sync);
    type Error = tokio_postgres::Error;
}

impl AsMut<tokio_postgres::Client> for Client {
    fn as_mut(&mut self) -> &mut tokio_postgres::Client {
        &mut self.client
    }
}

impl AsRef<tokio_postgres::Client> for Client {
    fn as_ref(&self) -> &tokio_postgres::Client {
        &self.client
    }
}

impl From<tokio_postgres::Client> for Client {
    fn from(client: tokio_postgres::Client) -> Self {
        Self::new(client)
    }
}

impl Client {
    pub fn new(client: tokio_postgres::Client) -> Self {
        let statements = std::collections::HashMap::new();
        Client { client, statements }
    }

    async fn prepare_internal<S: Into<String>>(
        &mut self,
        query_text: S,
    ) -> Result<tokio_postgres::Statement, Error<tokio_postgres::Error>> {
        match self.statements.entry(query_text.into()) {
            std::collections::hash_map::Entry::Occupied(entry) => Ok(entry.get().clone()),
            std::collections::hash_map::Entry::Vacant(entry) => {
                let statement = self
                    .client
                    .prepare(entry.key())
                    .await
                    .map_err(Error::prepare)?;
                Ok(entry.insert(statement).clone())
            }
        }
    }

    pub async fn query<Q: Query<Self>>(
        &mut self,
        query: &Q,
    ) -> Result<Vec<Q::Row>, Error<tokio_postgres::Error>> {
        let params = query.to_params();
        let statement = self.prepare_internal(query.query_text()).await?;

        let rows = self
            .client
            .query(&statement, &params)
            .await
            .map_err(Error::query)?;

        FromRow::from_rows(&rows)
    }

    pub async fn execute<S: Statement<Self>>(
        &mut self,
        statement: &S,
    ) -> Result<u64, Error<tokio_postgres::Error>> {
        let params = statement.to_params();
        let statement = self.prepare_internal(statement.query_text()).await?;

        let rows_affected = self
            .client
            .execute(&statement, &params)
            .await
            .map_err(Error::query)?;

        Ok(rows_affected)
    }

    pub async fn prepare<S: StaticQueryText>(&mut self) -> Result<(), Error<tokio_postgres::Error>> {
        self.prepare_internal(S::QUERY_TEXT).await?;
        Ok(())
    }

    pub async fn transaction(&mut self) -> Result<Transaction, Error<tokio_postgres::Error>> {
        Ok(Transaction {
            txn: self.client.transaction().await.map_err(Error::transaction)?,
            statements: &mut self.statements,
        })
    }
}

pub struct Transaction<'a> {
    txn: tokio_postgres::Transaction<'a>,
    statements: &'a mut std::collections::HashMap<String, tokio_postgres::Statement>,
}

impl<'a> Transaction<'a> {
    async fn prepare_internal<S: Into<String>>(
        &mut self,
        query_text: S,
    ) -> Result<tokio_postgres::Statement, Error<tokio_postgres::Error>> {
        match self.statements.entry(query_text.into()) {
            std::collections::hash_map::Entry::Occupied(entry) => Ok(entry.get().clone()),
            std::collections::hash_map::Entry::Vacant(entry) => {
                let statement = self
                    .txn
                    .prepare(entry.key())
                    .await
                    .map_err(Error::prepare)?;
                Ok(entry.insert(statement).clone())
            }
        }
    }

    pub async fn commit(self) -> Result<(), Error<tokio_postgres::Error>> {
        self.txn.commit().await.map_err(Error::transaction)
    }

    pub async fn rollback(self) -> Result<(), Error<tokio_postgres::Error>> {
        self.txn.rollback().await.map_err(Error::transaction)
    }

    pub async fn query<Q: Query<Client>>(
        &mut self,
        query: &Q,
    ) -> Result<Vec<Q::Row>, Error<tokio_postgres::Error>> {
        let params = query.to_params();
        let statement = self.prepare_internal(query.query_text()).await?;

        let rows = self
            .txn
            .query(&statement, &params)
            .await
            .map_err(Error::query)?;

        FromRow::from_rows(&rows)
    }

    pub async fn execute<S: Statement<Client>>(
        &mut self,
        statement: &S,
    ) -> Result<u64, Error<tokio_postgres::Error>> {
        let params = statement.to_params();
        let statement = self.prepare_internal(statement.query_text()).await?;

        let rows_affected = self
            .txn
            .execute(&statement, &params)
            .await
            .map_err(Error::query)?;

        Ok(rows_affected)
    }

    pub async fn prepare<S: StaticQueryText>(&mut self) -> Result<(), Error<tokio_postgres::Error>> {
        self.prepare_internal(S::QUERY_TEXT).await?;
        Ok(())
    }
}
