//! A synchronous client for PostgreSQL.

use crate::client::{FromColumnIndexed, FromColumnNamed, SyncClient, SyncTransaction, ToParam};
use crate::query::StaticQueryText;
use crate::{Error, FromRow, Query, Statement};

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
    client: postgres::Client,
    statements: std::collections::HashMap<String, tokio_postgres::Statement>,
}

impl Client {
    pub fn new(client: postgres::Client) -> Self {
        let statements = std::collections::HashMap::new();
        Client { client, statements }
    }

    /// A convenience function which parses a configuration string into a `Config` and then connects to the database.
    ///
    /// See the documentation for `postgres::Config` for information about the connection syntax.
    pub fn connect<T>(params: &str, tls_mode: T) -> Result<Self, tokio_postgres::Error>
    where
        T: postgres::tls::MakeTlsConnect<postgres::Socket> + 'static + Send,
        T::TlsConnect: Send,
        T::Stream: Send,
        <T::TlsConnect as postgres::tls::TlsConnect<postgres::Socket>>::Future: Send,
    {
        let client = postgres::Client::connect(params, tls_mode)?;
        Ok(Self::new(client))
    }

    fn prepare_internal<S: Into<String>>(
        &mut self,
        query_text: S,
    ) -> Result<postgres::Statement, Error<tokio_postgres::Error>> {
        match self.statements.entry(query_text.into()) {
            std::collections::hash_map::Entry::Occupied(entry) => Ok(entry.get().clone()),
            std::collections::hash_map::Entry::Vacant(entry) => {
                let statement = self
                    .client
                    .prepare(entry.key())
                    .map_err(Error::prepare)?;
                Ok(entry.insert(statement).clone())
            }
        }
    }
}

impl AsMut<postgres::Client> for Client {
    fn as_mut(&mut self) -> &mut postgres::Client {
        &mut self.client
    }
}

impl AsRef<postgres::Client> for Client {
    fn as_ref(&self) -> &postgres::Client {
        &self.client
    }
}

impl From<postgres::Client> for Client {
    fn from(client: postgres::Client) -> Self {
        Self::new(client)
    }
}

impl crate::client::Client for Client {
    type Row<'a> = tokio_postgres::Row;
    type Param<'a> = &'a (dyn tokio_postgres::types::ToSql + Sync);
    type Error = tokio_postgres::Error;
}

impl SyncClient for Client {
    fn query<Q: Query<Self>>(
        &mut self,
        query: &Q,
    ) -> Result<Vec<Q::Row>, Error<tokio_postgres::Error>> {
        let params = query.to_params();
        let statement = self.prepare_internal(query.query_text())?;

        let rows = self
            .client
            .query(&statement, &params)
            .map_err(Error::query)?;

        FromRow::from_rows(&rows)
    }

    fn execute<S: Statement<Self>>(
        &mut self,
        statement: &S,
    ) -> Result<u64, Error<tokio_postgres::Error>> {
        let params = statement.to_params();
        let statement = self.prepare_internal(statement.query_text())?;

        let rows_affected = self
            .client
            .execute(&statement, &params)
            .map_err(Error::query)?;

        Ok(rows_affected)
    }

    fn prepare<S: StaticQueryText>(&mut self) -> Result<(), Error<tokio_postgres::Error>> {
        self.prepare_internal(S::QUERY_TEXT)?;
        Ok(())
    }

    type Transaction<'a> = Transaction<'a>;

    fn transaction(&mut self) -> Result<Transaction, Error<tokio_postgres::Error>> {
        Ok(Transaction {
            txn: self.client.transaction().map_err(Error::transaction)?,
            statements: &mut self.statements,
        })
    }
}

pub struct Transaction<'a> {
    txn: postgres::Transaction<'a>,
    statements: &'a mut std::collections::HashMap<String, tokio_postgres::Statement>,
}

impl<'a> Transaction<'a> {
    fn prepare_internal<S: Into<String>>(
        &mut self,
        query_text: S,
    ) -> Result<tokio_postgres::Statement, Error<tokio_postgres::Error>> {
        match self.statements.entry(query_text.into()) {
            std::collections::hash_map::Entry::Occupied(entry) => Ok(entry.get().clone()),
            std::collections::hash_map::Entry::Vacant(entry) => {
                let statement = self
                    .txn
                    .prepare(entry.key())
                    .map_err(Error::prepare)?;
                Ok(entry.insert(statement).clone())
            }
        }
    }
}

impl<'a> SyncTransaction<Client> for Transaction<'a> {
    fn commit(self) -> Result<(), Error<tokio_postgres::Error>> {
        self.txn.commit().map_err(Error::transaction)
    }

    fn rollback(self) -> Result<(), Error<tokio_postgres::Error>> {
        self.txn.rollback().map_err(Error::transaction)
    }

    fn query<Q: Query<Client>>(
        &mut self,
        query: &Q,
    ) -> Result<Vec<Q::Row>, Error<tokio_postgres::Error>> {
        let params = query.to_params();
        let statement = self.prepare_internal(query.query_text())?;

        let rows = self
            .txn
            .query(&statement, &params)
            .map_err(Error::query)?;

        FromRow::from_rows(&rows)
    }

    fn execute<S: Statement<Client>>(
        &mut self,
        statement: &S,
    ) -> Result<u64, Error<tokio_postgres::Error>> {
        let params = statement.to_params();
        let statement = self.prepare_internal(statement.query_text())?;

        let rows_affected = self
            .txn
            .execute(&statement, &params)
            .map_err(Error::query)?;

        Ok(rows_affected)
    }

    fn prepare<S: StaticQueryText>(&mut self) -> Result<(), Error<tokio_postgres::Error>> {
        self.prepare_internal(S::QUERY_TEXT)?;
        Ok(())
    }
}
