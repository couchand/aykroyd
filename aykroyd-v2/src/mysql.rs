//! MySQL bindings.

use crate::client::{Client, FromColumnIndexed, FromColumnNamed, SyncClient, ToParam};
use crate::error::Error;
use crate::query::{Query, Statement, StaticQueryText};
use crate::row::FromRow;

impl<T> FromColumnIndexed<mysql::Conn> for T
where
    T: mysql::prelude::FromValue,
{
    fn from_column(row: &mysql::Row, index: usize) -> Result<Self, Error> {
        row.get_opt(index)
            .ok_or_else(|| Error::FromColumn(format!("unknown column {}", index)))?
            .map_err(|e| Error::FromColumn(e.to_string()))
    }
}

impl<T> FromColumnNamed<mysql::Conn> for T
where
    T: mysql::prelude::FromValue,
{
    fn from_column(row: &mysql::Row, name: &str) -> Result<Self, Error> {
        row.get_opt(name)
            .ok_or_else(|| Error::FromColumn(format!("unknown column {}", name)))?
            .map_err(|e| Error::FromColumn(e.to_string()))
    }
}

impl<T> ToParam<mysql::Conn> for T
where
    T: Into<mysql::Value> + Clone,
{
    fn to_param(&self) -> mysql::Value {
        self.clone().into()
    }
}

impl Client for mysql::Conn {
    type Row<'a> = mysql::Row;
    type Param<'a> = mysql::Value;
}

impl SyncClient for mysql::Conn {
    fn query<Q: Query<Self>>(&mut self, query: &Q) -> Result<Vec<Q::Row>, Error> {
        use mysql::prelude::Queryable;

        let params = query.to_params();
        let params = match params.len() {
            0 => mysql::Params::Empty,
            _ => mysql::Params::Positional(params),
        };
        let query = self
            .prep(query.query_text())
            .map_err(|e| Error::Prepare(e.to_string()))?;

        let rows: Vec<mysql::Row> = mysql::prelude::Queryable::exec(self, &query, params)
            .map_err(|e| Error::Query(e.to_string()))?;

        FromRow::from_rows(&rows)
    }

    fn execute<S: Statement<Self>>(&mut self, statement: &S) -> Result<u64, Error> {
        use mysql::prelude::Queryable;

        let params = statement.to_params();
        let params = match params.len() {
            0 => mysql::Params::Empty,
            _ => mysql::Params::Positional(params),
        };
        let statement = self
            .prep(statement.query_text())
            .map_err(|e| Error::Prepare(e.to_string()))?;

        mysql::prelude::Queryable::exec_drop(self, &statement, params)
            .map_err(|e| Error::Query(e.to_string()))?;

        Ok(self.affected_rows())
    }

    fn prepare<S: StaticQueryText>(&mut self) -> Result<(), Error> {
        use mysql::prelude::Queryable;
        self.prep(S::QUERY_TEXT)
            .map_err(|e| Error::Prepare(e.to_string()))?;
        Ok(())
    }
}
