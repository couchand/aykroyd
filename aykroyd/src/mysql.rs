//! MySQL bindings.

use crate::client::{FromColumnIndexed, FromColumnNamed, ToParam};
use crate::query::StaticQueryText;
use crate::{error, FromRow, Query, QueryOne, Statement};

pub type Error = error::Error<mysql::Error>;

impl<T> FromColumnIndexed<Client> for T
where
    T: mysql::prelude::FromValue,
{
    fn from_column(row: &mysql::Row, index: usize) -> Result<Self, Error> {
        row.get_opt(index)
            .ok_or_else(|| Error::from_column_str(format!("unknown column {}", index), None))?
            .map_err(|e| Error::from_column_str(e.to_string(), None))
    }
}

impl<T> FromColumnNamed<Client> for T
where
    T: mysql::prelude::FromValue,
{
    fn from_column(row: &mysql::Row, name: &str) -> Result<Self, Error> {
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

impl crate::client::Client for Client {
    type Row<'a> = mysql::Row;
    type Param<'a> = mysql::Value;
    type Error = mysql::Error;
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

impl Client {
    pub fn new<T, E>(opts: T) -> Result<Self, Error>
    where
        mysql::Opts: TryFrom<T, Error = E>,
        mysql::Error: From<E>,
    {
        mysql::Conn::new(opts).map(Client).map_err(Error::connect)
    }

    pub fn query<Q: Query<Self>>(&mut self, query: &Q) -> Result<Vec<Q::Row>, Error> {
        use mysql::prelude::Queryable;

        let params = match query.to_params() {
            None => mysql::Params::Empty,
            Some(params) => mysql::Params::Positional(params),
        };
        let query = self
            .as_mut()
            .prep(query.query_text())
            .map_err(Error::prepare)?;

        let rows: Vec<mysql::Row> =
            mysql::prelude::Queryable::exec(self.as_mut(), &query, params).map_err(Error::query)?;

        FromRow::from_rows(&rows)
    }

    pub fn query_one<Q: QueryOne<Self>>(&mut self, query: &Q) -> Result<Q::Row, Error> {
        use mysql::prelude::Queryable;

        let params = match query.to_params() {
            None => mysql::Params::Empty,
            Some(params) => mysql::Params::Positional(params),
        };
        let query = self
            .as_mut()
            .prep(query.query_text())
            .map_err(Error::prepare)?;

        let row: Option<mysql::Row> =
            mysql::prelude::Queryable::exec_first(self.as_mut(), &query, params).map_err(Error::query)?;

        row
            .ok_or_else(|| Error::query_str("query returned no rows", None))
            .and_then(|row| FromRow::from_row(&row))
    }

    pub fn query_opt<Q: QueryOne<Self>>(&mut self, query: &Q) -> Result<Option<Q::Row>, Error> {
        use mysql::prelude::Queryable;

        let params = match query.to_params() {
            None => mysql::Params::Empty,
            Some(params) => mysql::Params::Positional(params),
        };
        let query = self
            .as_mut()
            .prep(query.query_text())
            .map_err(Error::prepare)?;

        let row: Option<mysql::Row> =
            mysql::prelude::Queryable::exec_first(self.as_mut(), &query, params).map_err(Error::query)?;

        row.map(|row| FromRow::from_row(&row)).transpose()
    }

    pub fn execute<S: Statement<Self>>(
        &mut self,
        statement: &S,
    ) -> Result<u64, Error> {
        use mysql::prelude::Queryable;

        let params = match statement.to_params() {
            None => mysql::Params::Empty,
            Some(params) => mysql::Params::Positional(params),
        };
        let statement = self
            .as_mut()
            .prep(statement.query_text())
            .map_err(Error::prepare)?;

        mysql::prelude::Queryable::exec_drop(self.as_mut(), &statement, params)
            .map_err(Error::query)?;

        Ok(self.0.affected_rows())
    }

    pub fn prepare<S: StaticQueryText>(&mut self) -> Result<(), Error> {
        use mysql::prelude::Queryable;
        self.0.prep(S::QUERY_TEXT).map_err(Error::prepare)?;
        Ok(())
    }

    pub fn transaction(&mut self) -> Result<Transaction<'_>, Error> {
        Ok(Transaction(
            self.0
                .start_transaction(mysql::TxOpts::default())
                .map_err(Error::transaction)?,
        ))
    }
}

pub struct Transaction<'a>(mysql::Transaction<'a>);

impl<'a> Transaction<'a> {
    pub fn commit(self) -> Result<(), Error> {
        self.0.commit().map_err(Error::transaction)
    }

    pub fn rollback(self) -> Result<(), Error> {
        self.0.rollback().map_err(Error::transaction)
    }

    pub fn query<Q: Query<Client>>(
        &mut self,
        query: &Q,
    ) -> Result<Vec<Q::Row>, Error> {
        use mysql::prelude::Queryable;

        let params = match query.to_params() {
            None => mysql::Params::Empty,
            Some(params) => mysql::Params::Positional(params),
        };
        let query = self.0.prep(query.query_text()).map_err(Error::prepare)?;

        let rows: Vec<mysql::Row> =
            mysql::prelude::Queryable::exec(&mut self.0, &query, params).map_err(Error::query)?;

        FromRow::from_rows(&rows)
    }

    pub fn query_one<Q: QueryOne<Client>>(&mut self, query: &Q) -> Result<Q::Row, Error> {
        use mysql::prelude::Queryable;

        let params = match query.to_params() {
            None => mysql::Params::Empty,
            Some(params) => mysql::Params::Positional(params),
        };
        let query = self.0
            .prep(query.query_text())
            .map_err(Error::prepare)?;

        let row: Option<mysql::Row> =
            mysql::prelude::Queryable::exec_first(&mut self.0, &query, params).map_err(Error::query)?;

        row
            .ok_or_else(|| Error::query_str("query returned no rows", None))
            .and_then(|row| FromRow::from_row(&row))
    }

    pub fn query_opt<Q: QueryOne<Client>>(&mut self, query: &Q) -> Result<Option<Q::Row>, Error> {
        use mysql::prelude::Queryable;

        let params = match query.to_params() {
            None => mysql::Params::Empty,
            Some(params) => mysql::Params::Positional(params),
        };
        let query = self.0
            .prep(query.query_text())
            .map_err(Error::prepare)?;

        let row: Option<mysql::Row> =
            mysql::prelude::Queryable::exec_first(&mut self.0, &query, params).map_err(Error::query)?;

        row.map(|row| FromRow::from_row(&row)).transpose()
    }

    pub fn execute<S: Statement<Client>>(
        &mut self,
        statement: &S,
    ) -> Result<u64, Error> {
        use mysql::prelude::Queryable;

        let params = match statement.to_params() {
            None => mysql::Params::Empty,
            Some(params) => mysql::Params::Positional(params),
        };
        let statement = self
            .0
            .prep(statement.query_text())
            .map_err(Error::prepare)?;

        mysql::prelude::Queryable::exec_drop(&mut self.0, &statement, params)
            .map_err(Error::query)?;

        Ok(self.0.affected_rows())
    }

    pub fn prepare<S: StaticQueryText>(&mut self) -> Result<(), Error> {
        use mysql::prelude::Queryable;
        self.0.prep(S::QUERY_TEXT).map_err(Error::prepare)?;
        Ok(())
    }
}

// TODO: not derive support
#[cfg(all(test, feature ="derive"))]
mod test {
    use super::*;

    #[derive(Statement)]
    #[aykroyd(text = "CREATE TABLE test_mysql (id SERIAL PRIMARY KEY, label TEXT NOT NULL)")]
    struct CreateTodos;

    #[derive(Statement)]
    #[aykroyd(text = "DROP TABLE test_mysql")]
    struct DropTodos;

    #[derive(Statement)]
    #[aykroyd(text = "INSERT INTO test_mysql (label) VALUES (?)")]
    struct InsertTodo<'a>(&'a str);

    #[derive(Query)]
    #[aykroyd(row((i32, String)), text = "SELECT id, label FROM test_mysql")]
    struct GetAllTodos;

    #[test]
    fn end_to_end() {
        const TODO_TEXT: &str = "get things done, please!";

        let mut client = Client::new(
            "mysql://aykroyd_test:aykroyd_test@localhost:3306/aykroyd_test"
        ).unwrap();

        client.execute(&CreateTodos).unwrap();

        client.execute(&InsertTodo(TODO_TEXT)).unwrap();

        let todos = client.query(&GetAllTodos).unwrap();
        assert_eq!(1, todos.len());
        assert_eq!(TODO_TEXT, todos[0].1);

        client.execute(&DropTodos).unwrap();
    }
}
