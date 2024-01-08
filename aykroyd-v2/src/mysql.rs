//! MySQL bindings.

use crate::client::{Client, FromColumnIndexed, FromColumnNamed, SyncClient, ToParam};
use crate::error::Error;
use crate::query::{Query, Statement, StaticQueryText};
use crate::row::FromRow;

impl<T> FromColumnIndexed<mysql::Conn> for T
where
    T: mysql::prelude::FromValue,
{
    fn from_column(row: &mysql::Row, index: usize) -> Result<Self, Error<mysql::Error>> {
        row.get_opt(index)
            .ok_or_else(|| Error::from_column_str(format!("unknown column {}", index), None))?
            .map_err(|e| Error::from_column_str(e.to_string(), None))
    }
}

impl<T> FromColumnNamed<mysql::Conn> for T
where
    T: mysql::prelude::FromValue,
{
    fn from_column(row: &mysql::Row, name: &str) -> Result<Self, Error<mysql::Error>> {
        row.get_opt(name)
            .ok_or_else(|| Error::from_column_str(format!("unknown column {}", name), None))?
            .map_err(|e| Error::from_column_str(e.to_string(), None))
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
    type Error = mysql::Error;
}

impl SyncClient for mysql::Conn {
    fn query<Q: Query<Self>>(&mut self, query: &Q) -> Result<Vec<Q::Row>, Error<mysql::Error>> {
        use mysql::prelude::Queryable;

        let params = query.to_params();
        let params = match params.len() {
            0 => mysql::Params::Empty,
            _ => mysql::Params::Positional(params),
        };
        let query = self.prep(query.query_text()).map_err(Error::prepare)?;

        let rows: Vec<mysql::Row> =
            mysql::prelude::Queryable::exec(self, &query, params).map_err(Error::query)?;

        FromRow::from_rows(&rows)
    }

    fn execute<S: Statement<Self>>(&mut self, statement: &S) -> Result<u64, Error<mysql::Error>> {
        use mysql::prelude::Queryable;

        let params = statement.to_params();
        let params = match params.len() {
            0 => mysql::Params::Empty,
            _ => mysql::Params::Positional(params),
        };
        let statement = self.prep(statement.query_text()).map_err(Error::prepare)?;

        mysql::prelude::Queryable::exec_drop(self, &statement, params).map_err(Error::query)?;

        Ok(self.affected_rows())
    }

    fn prepare<S: StaticQueryText>(&mut self) -> Result<(), Error<mysql::Error>> {
        use mysql::prelude::Queryable;
        self.prep(S::QUERY_TEXT).map_err(Error::prepare)?;
        Ok(())
    }
}
