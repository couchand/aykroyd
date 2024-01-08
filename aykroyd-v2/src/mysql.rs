//! MySQL bindings.

use crate::client::{FromColumnIndexed, FromColumnNamed, SyncClient, ToParam};
use crate::error::Error;
use crate::query::{Query, Statement, StaticQueryText};
use crate::row::FromRow;

impl<T> FromColumnIndexed<Client> for T
where
    T: mysql::prelude::FromValue,
{
    fn from_column(row: &mysql::Row, index: usize) -> Result<Self, Error<mysql::Error>> {
        row.get_opt(index)
            .ok_or_else(|| Error::from_column_str(format!("unknown column {}", index), None))?
            .map_err(|e| Error::from_column_str(e.to_string(), None))
    }
}

impl<T> FromColumnNamed<Client> for T
where
    T: mysql::prelude::FromValue,
{
    fn from_column(row: &mysql::Row, name: &str) -> Result<Self, Error<mysql::Error>> {
        row.get_opt(name)
            .ok_or_else(|| Error::from_column_str(format!("unknown column {}", name), None))?
            .map_err(|e| Error::from_column_str(e.to_string(), None))
    }
}

impl<T> ToParam<Client> for T
where
    T: Into<mysql::Value> + Clone,
{
    fn to_param(&self) -> mysql::Value {
        self.clone().into()
    }
}

pub struct Client(mysql::Conn);

impl Client {
    pub fn new<T, E>(opts: T) -> Result<Self, mysql::Error>
    where
        mysql::Opts: TryFrom<T, Error = E>,
        mysql::Error: From<E>,
    {
        mysql::Conn::new(opts).map(Client)
    }
}

impl AsMut<mysql::Conn> for Client {
    fn as_mut(&mut self) -> &mut mysql::Conn {
        &mut self.0
    }
}

impl AsRef<mysql::Conn> for Client {
    fn as_ref(&self) -> &mysql::Conn {
        &self.0
    }
}

impl From<mysql::Conn> for Client {
    fn from(inner: mysql::Conn) -> Self {
        Client(inner)
    }
}

impl crate::client::Client for Client {
    type Row<'a> = mysql::Row;
    type Param<'a> = mysql::Value;
    type Error = mysql::Error;
}

impl SyncClient for Client {
    fn query<Q: Query<Self>>(&mut self, query: &Q) -> Result<Vec<Q::Row>, Error<mysql::Error>> {
        use mysql::prelude::Queryable;

        let params = query.to_params();
        let params = match params.len() {
            0 => mysql::Params::Empty,
            _ => mysql::Params::Positional(params),
        };
        let query = self.as_mut().prep(query.query_text()).map_err(Error::prepare)?;

        let rows: Vec<mysql::Row> =
            mysql::prelude::Queryable::exec(self.as_mut(), &query, params).map_err(Error::query)?;

        FromRow::from_rows(&rows)
    }

    fn execute<S: Statement<Self>>(&mut self, statement: &S) -> Result<u64, Error<mysql::Error>> {
        use mysql::prelude::Queryable;

        let params = statement.to_params();
        let params = match params.len() {
            0 => mysql::Params::Empty,
            _ => mysql::Params::Positional(params),
        };
        let statement = self.as_mut().prep(statement.query_text()).map_err(Error::prepare)?;

        mysql::prelude::Queryable::exec_drop(self.as_mut(), &statement, params).map_err(Error::query)?;

        Ok(self.0.affected_rows())
    }

    fn prepare<S: StaticQueryText>(&mut self) -> Result<(), Error<mysql::Error>> {
        use mysql::prelude::Queryable;
        self.0.prep(S::QUERY_TEXT).map_err(Error::prepare)?;
        Ok(())
    }
}
