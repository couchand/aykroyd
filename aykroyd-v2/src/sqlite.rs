//! Sqlite bindings.

use super::client::SyncClient;
use super::query::ToParam;
use super::{Client, Error, FromRow, FromColumn, Query, Statement, StaticQueryText};

impl<'a, T> FromColumn<&rusqlite::Row<'a>, usize> for T
where
    T: rusqlite::types::FromSql,
{
    fn get(row: &rusqlite::Row, index: usize) -> Result<Self, Error> {
        row.get(index).map_err(|e| Error::FromColumn(e.to_string()))
    }
}

impl<'a, T> FromColumn<&rusqlite::Row<'a>, &str> for T
where
    T: rusqlite::types::FromSql,
{
    fn get(row: &rusqlite::Row, name: &str) -> Result<Self, Error> {
        row.get(name).map_err(|e| Error::FromColumn(e.to_string()))
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
}

impl SyncClient for rusqlite::Connection {
    fn query<Q: Query<Self>>(&mut self, query: &Q) -> Result<Vec<Q::Row>, Error> {
        let params = query.to_params();

        let mut statement = rusqlite::Connection::prepare_cached(self, &query.query_text())
            .map_err(|e| Error::Prepare(e.to_string()))?;

        let mut rows = statement
            .query(&params[..])
            .map_err(|e| Error::Query(e.to_string()))?;

        let mut result = vec![];
        while let Some(row) = rows.next().map_err(|e| Error::Query(e.to_string()))? {
            result.push(FromRow::from_row(row)?);
        }

        Ok(result)
    }

    fn execute<S: Statement<Self>>(&mut self, statement: &S) -> Result<u64, Error> {
        let params = statement.to_params();

        let mut statement = rusqlite::Connection::prepare_cached(self, &statement.query_text())
            .map_err(|e| Error::Prepare(e.to_string()))?;

        let rows_affected = statement
            .execute(&params[..])
            .map_err(|e| Error::Query(e.to_string()))?;

        Ok(rows_affected.try_into().unwrap_or_default())
    }

    fn prepare<S: StaticQueryText>(&mut self) -> Result<(), Error> {
        self.prepare_cached(S::QUERY_TEXT)
            .map_err(|e| Error::Prepare(e.to_string()))?;
        Ok(())
    }
}
