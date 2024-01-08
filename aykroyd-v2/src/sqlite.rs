//! Sqlite bindings.

use crate::client::{Client, FromColumnIndexed, FromColumnNamed, SyncClient, ToParam};
use crate::error::Error;
use crate::query::{Query, Statement, StaticQueryText};
use crate::row::FromRow;

impl<T> FromColumnIndexed<rusqlite::Connection> for T
where
    T: rusqlite::types::FromSql,
{
    fn from_column(row: &rusqlite::Row, index: usize) -> Result<Self, Error<rusqlite::Error>> {
        row.get(index).map_err(Error::from_column)
    }
}

impl<T> FromColumnNamed<rusqlite::Connection> for T
where
    T: rusqlite::types::FromSql,
{
    fn from_column(row: &rusqlite::Row, name: &str) -> Result<Self, Error<rusqlite::Error>> {
        row.get(name).map_err(Error::from_column)
    }
}

impl<T> ToParam<rusqlite::Connection> for T
where
    T: rusqlite::types::ToSql,
{
    fn to_param(&self) -> &dyn rusqlite::types::ToSql {
        self
    }
}

impl Client for rusqlite::Connection {
    type Row<'a> = rusqlite::Row<'a>;
    type Param<'a> = &'a dyn rusqlite::types::ToSql;
    type Error = rusqlite::Error;
}

impl SyncClient for rusqlite::Connection {
    fn query<Q: Query<Self>>(&mut self, query: &Q) -> Result<Vec<Q::Row>, Error<rusqlite::Error>> {
        let params = query.to_params();

        let mut statement = rusqlite::Connection::prepare_cached(self, &query.query_text())
            .map_err(Error::prepare)?;

        let mut rows = statement
            .query(&params[..])
            .map_err(Error::query)?;

        let mut result = vec![];
        while let Some(row) = rows.next().map_err(Error::query)? {
            result.push(FromRow::from_row(row)?);
        }

        Ok(result)
    }

    fn execute<S: Statement<Self>>(&mut self, statement: &S) -> Result<u64, Error<rusqlite::Error>> {
        let params = statement.to_params();

        let mut statement = rusqlite::Connection::prepare_cached(self, &statement.query_text())
            .map_err(Error::prepare)?;

        let rows_affected = statement
            .execute(&params[..])
            .map_err(Error::query)?;

        Ok(rows_affected.try_into().unwrap_or_default())
    }

    fn prepare<S: StaticQueryText>(&mut self) -> Result<(), Error<rusqlite::Error>> {
        self.prepare_cached(S::QUERY_TEXT)
            .map_err(Error::prepare)?;
        Ok(())
    }
}
