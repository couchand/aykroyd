//! MySQL bindings.

use super::client::SyncClient;
use super::{Client, Error, FromRow, FromSql, Query, Statement, StaticQueryText};

impl<T: mysql::prelude::FromValue> FromSql<&mysql::Row, usize> for T {
    fn get(row: &mysql::Row, index: usize) -> Result<Self, Error> {
        row.get_opt(index)
            .ok_or_else(|| Error::FromSql(format!("unknown column {}", index)))?
            .map_err(|e| Error::FromSql(e.to_string()))
    }
}

impl<T: mysql::prelude::FromValue> FromSql<&mysql::Row, &str> for T {
    fn get(row: &mysql::Row, name: &str) -> Result<Self, Error> {
        row.get_opt(name)
            .ok_or_else(|| Error::FromSql(format!("unknown column {}", name)))?
            .map_err(|e| Error::FromSql(e.to_string()))
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

        let rows = mysql::prelude::Queryable::exec(self, &query, params)
            .map_err(|e| Error::Query(e.to_string()))?;

        rows.iter().map(FromRow::from_row).collect()
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
