//! Sqlite bindings.

use crate::client::{FromColumnIndexed, FromColumnNamed, ToParam};
use crate::query::StaticQueryText;
use crate::{Error, FromRow, Query, Statement};

impl<T> FromColumnIndexed<Client> for T
where
    T: rusqlite::types::FromSql,
{
    fn from_column(row: &rusqlite::Row, index: usize) -> Result<Self, Error<rusqlite::Error>> {
        row.get(index).map_err(Error::from_column)
    }
}

impl<T> FromColumnNamed<Client> for T
where
    T: rusqlite::types::FromSql,
{
    fn from_column(row: &rusqlite::Row, name: &str) -> Result<Self, Error<rusqlite::Error>> {
        row.get(name).map_err(Error::from_column)
    }
}

impl<T> ToParam<Client> for T
where
    T: rusqlite::types::ToSql,
{
    fn to_param(&self) -> &dyn rusqlite::types::ToSql {
        self
    }
}

pub struct Client(rusqlite::Connection);

impl crate::client::Client for Client {
    type Row<'a> = rusqlite::Row<'a>;
    type Param<'a> = &'a dyn rusqlite::types::ToSql;
    type Error = rusqlite::Error;
}

impl AsMut<rusqlite::Connection> for Client {
    fn as_mut(&mut self) -> &mut rusqlite::Connection {
        &mut self.0
    }
}

impl AsRef<rusqlite::Connection> for Client {
    fn as_ref(&self) -> &rusqlite::Connection {
        &self.0
    }
}

impl From<rusqlite::Connection> for Client {
    fn from(inner: rusqlite::Connection) -> Self {
        Client(inner)
    }
}

impl Client {
    pub fn open<P: AsRef<std::path::Path>>(path: P) -> Result<Self, rusqlite::Error> {
        rusqlite::Connection::open(path).map(Client)
    }

    pub fn open_in_memory() -> Result<Self, rusqlite::Error> {
        rusqlite::Connection::open_in_memory().map(Client)
    }

    pub fn query<Q: Query<Self>>(
        &mut self,
        query: &Q,
    ) -> Result<Vec<Q::Row>, Error<rusqlite::Error>> {
        let params = query.to_params();

        let mut statement =
            rusqlite::Connection::prepare_cached(self.as_mut(), &query.query_text())
                .map_err(Error::prepare)?;

        let mut rows = statement.query(&params[..]).map_err(Error::query)?;

        let mut result = vec![];
        while let Some(row) = rows.next().map_err(Error::query)? {
            result.push(FromRow::from_row(row)?);
        }

        Ok(result)
    }

    pub fn execute<S: Statement<Self>>(
        &mut self,
        statement: &S,
    ) -> Result<u64, Error<rusqlite::Error>> {
        let params = statement.to_params();

        let mut statement =
            rusqlite::Connection::prepare_cached(self.as_mut(), &statement.query_text())
                .map_err(Error::prepare)?;

        let rows_affected = statement.execute(&params[..]).map_err(Error::query)?;

        Ok(rows_affected.try_into().unwrap_or_default())
    }

    pub fn prepare<S: StaticQueryText>(&mut self) -> Result<(), Error<rusqlite::Error>> {
        self.as_mut()
            .prepare_cached(S::QUERY_TEXT)
            .map_err(Error::prepare)?;
        Ok(())
    }

    pub fn transaction(&mut self) -> Result<Transaction<'_>, Error<rusqlite::Error>> {
        Ok(Transaction(
            self.0.transaction().map_err(Error::transaction)?,
        ))
    }
}

pub struct Transaction<'a>(rusqlite::Transaction<'a>);

impl<'a> Transaction<'a> {
    pub fn commit(self) -> Result<(), Error<rusqlite::Error>> {
        self.0.commit().map_err(Error::transaction)
    }

    pub fn rollback(self) -> Result<(), Error<rusqlite::Error>> {
        self.0.rollback().map_err(Error::transaction)
    }

    pub fn query<Q: Query<Client>>(
        &mut self,
        query: &Q,
    ) -> Result<Vec<Q::Row>, Error<rusqlite::Error>> {
        let params = query.to_params();

        let mut statement = rusqlite::Connection::prepare_cached(&self.0, &query.query_text())
            .map_err(Error::prepare)?;

        let mut rows = statement.query(&params[..]).map_err(Error::query)?;

        let mut result = vec![];
        while let Some(row) = rows.next().map_err(Error::query)? {
            result.push(FromRow::from_row(row)?);
        }

        Ok(result)
    }

    pub fn execute<S: Statement<Client>>(
        &mut self,
        statement: &S,
    ) -> Result<u64, Error<rusqlite::Error>> {
        let params = statement.to_params();

        let mut statement = rusqlite::Connection::prepare_cached(&self.0, &statement.query_text())
            .map_err(Error::prepare)?;

        let rows_affected = statement.execute(&params[..]).map_err(Error::query)?;

        Ok(rows_affected.try_into().unwrap_or_default())
    }

    pub fn prepare<S: StaticQueryText>(&mut self) -> Result<(), Error<rusqlite::Error>> {
        self.0
            .prepare_cached(S::QUERY_TEXT)
            .map_err(Error::prepare)?;
        Ok(())
    }
}
