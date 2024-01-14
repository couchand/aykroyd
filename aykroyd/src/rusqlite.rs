//! Sqlite bindings.

use crate::client::{FromColumnIndexed, FromColumnNamed, ToParam};
use crate::query::StaticQueryText;
use crate::{error, FromRow, Query, QueryOne, Statement};

pub type Error = error::Error<rusqlite::Error>;

impl<T> FromColumnIndexed<Client> for T
where
    T: rusqlite::types::FromSql,
{
    fn from_column(row: &rusqlite::Row, index: usize) -> Result<Self, Error> {
        row.get(index).map_err(Error::from_column)
    }
}

impl<T> FromColumnNamed<Client> for T
where
    T: rusqlite::types::FromSql,
{
    fn from_column(row: &rusqlite::Row, name: &str) -> Result<Self, Error> {
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
    pub fn open<P: AsRef<std::path::Path>>(path: P) -> Result<Self, Error> {
        rusqlite::Connection::open(path).map(Client).map_err(Error::connect)
    }

    pub fn open_in_memory() -> Result<Self, Error> {
        rusqlite::Connection::open_in_memory().map(Client).map_err(Error::connect)
    }

    pub fn query<Q: Query<Self>>(
        &mut self,
        query: &Q,
    ) -> Result<Vec<Q::Row>, Error> {
        let params = query.to_params();
        let params: &[_] = params.as_ref().map(AsRef::as_ref).unwrap_or(&[][..]);

        let mut statement =
            rusqlite::Connection::prepare_cached(self.as_mut(), &query.query_text())
                .map_err(Error::prepare)?;

        let mut rows = statement.query(params).map_err(Error::query)?;

        let mut result = vec![];
        while let Some(row) = rows.next().map_err(Error::query)? {
            result.push(FromRow::from_row(row)?);
        }

        Ok(result)
    }

    pub fn query_one<Q: QueryOne<Self>>(
        &mut self,
        query: &Q,
    ) -> Result<Q::Row, Error> {
        let params = query.to_params();
        let params: &[_] = params.as_ref().map(AsRef::as_ref).unwrap_or(&[][..]);

        let mut statement =
            rusqlite::Connection::prepare_cached(self.as_mut(), &query.query_text())
                .map_err(Error::prepare)?;

        let mut rows = statement.query(params).map_err(Error::query)?;

        rows.next()
            .map_err(Error::query)?
            .ok_or_else(|| Error::query(rusqlite::Error::QueryReturnedNoRows))
            .and_then(|row| FromRow::from_row(row))
    }

    pub fn query_opt<Q: QueryOne<Self>>(
        &mut self,
        query: &Q,
    ) -> Result<Option<Q::Row>, Error> {
        let params = query.to_params();
        let params: &[_] = params.as_ref().map(AsRef::as_ref).unwrap_or(&[][..]);

        let mut statement =
            rusqlite::Connection::prepare_cached(self.as_mut(), &query.query_text())
                .map_err(Error::prepare)?;

        let mut rows = statement.query(params).map_err(Error::query)?;

        rows.next()
            .map_err(Error::query)?
            .map(|row| FromRow::from_row(row))
            .transpose()
    }

    pub fn execute<S: Statement<Self>>(
        &mut self,
        statement: &S,
    ) -> Result<u64, Error> {
        let params = statement.to_params();
        let params: &[_] = params.as_ref().map(AsRef::as_ref).unwrap_or(&[][..]);

        let mut statement =
            rusqlite::Connection::prepare_cached(self.as_mut(), &statement.query_text())
                .map_err(Error::prepare)?;

        let rows_affected = statement.execute(params).map_err(Error::query)?;

        Ok(rows_affected.try_into().unwrap_or_default())
    }

    pub fn prepare<S: StaticQueryText>(&mut self) -> Result<(), Error> {
        self.as_mut()
            .prepare_cached(S::QUERY_TEXT)
            .map_err(Error::prepare)?;
        Ok(())
    }

    pub fn transaction(&mut self) -> Result<Transaction<'_>, Error> {
        Ok(Transaction(
            self.0.transaction().map_err(Error::transaction)?,
        ))
    }
}

pub struct Transaction<'a>(rusqlite::Transaction<'a>);

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
        let params = query.to_params();
        let params: &[_] = params.as_ref().map(AsRef::as_ref).unwrap_or(&[][..]);

        let mut statement = rusqlite::Connection::prepare_cached(&self.0, &query.query_text())
            .map_err(Error::prepare)?;

        let mut rows = statement.query(params).map_err(Error::query)?;

        let mut result = vec![];
        while let Some(row) = rows.next().map_err(Error::query)? {
            result.push(FromRow::from_row(row)?);
        }

        Ok(result)
    }

    pub fn query_one<Q: QueryOne<Client>>(
        &mut self,
        query: &Q,
    ) -> Result<Q::Row, Error> {
        let params = query.to_params();
        let params: &[_] = params.as_ref().map(AsRef::as_ref).unwrap_or(&[][..]);

        let mut statement =
            rusqlite::Connection::prepare_cached(&self.0, &query.query_text())
                .map_err(Error::prepare)?;

        let mut rows = statement.query(params).map_err(Error::query)?;

        rows.next()
            .map_err(Error::query)?
            .ok_or_else(|| Error::query(rusqlite::Error::QueryReturnedNoRows))
            .and_then(|row| FromRow::from_row(row))
    }

    pub fn query_opt<Q: QueryOne<Client>>(
        &mut self,
        query: &Q,
    ) -> Result<Option<Q::Row>, Error> {
        let params = query.to_params();
        let params: &[_] = params.as_ref().map(AsRef::as_ref).unwrap_or(&[][..]);

        let mut statement =
            rusqlite::Connection::prepare_cached(&self.0, &query.query_text())
                .map_err(Error::prepare)?;

        let mut rows = statement.query(params).map_err(Error::query)?;

        rows.next()
            .map_err(Error::query)?
            .map(|row| FromRow::from_row(row))
            .transpose()
    }

    pub fn execute<S: Statement<Client>>(
        &mut self,
        statement: &S,
    ) -> Result<u64, Error> {
        let params = statement.to_params();
        let params: &[_] = params.as_ref().map(AsRef::as_ref).unwrap_or(&[][..]);

        let mut statement = rusqlite::Connection::prepare_cached(&self.0, &statement.query_text())
            .map_err(Error::prepare)?;

        let rows_affected = statement.execute(params).map_err(Error::query)?;

        Ok(rows_affected.try_into().unwrap_or_default())
    }

    pub fn prepare<S: StaticQueryText>(&mut self) -> Result<(), Error> {
        self.0
            .prepare_cached(S::QUERY_TEXT)
            .map_err(Error::prepare)?;
        Ok(())
    }
}

// TODO: not derive support
#[cfg(all(test, feature ="derive"))]
mod test {
    use super::*;

    #[derive(Statement)]
    #[aykroyd(text = "CREATE TABLE test_rusqlite (id INTEGER PRIMARY KEY AUTOINCREMENT, label TEXT NOT NULL)")]
    struct CreateTodos;

    #[derive(Statement)]
    #[aykroyd(text = "DROP TABLE test_rusqlite")]
    struct DropTodos;

    #[derive(Statement)]
    #[aykroyd(text = "INSERT INTO test_rusqlite (label) VALUES ($1)")]
    struct InsertTodo<'a>(&'a str);

    #[derive(Query)]
    #[aykroyd(row((i32, String)), text = "SELECT id, label FROM test_rusqlite")]
    struct GetAllTodos;

    #[test]
    fn end_to_end_memory() {
        const TODO_TEXT: &str = "get things done, please!";

        let mut client = Client::open_in_memory().unwrap();

        client.execute(&CreateTodos).unwrap();

        client.execute(&InsertTodo(TODO_TEXT)).unwrap();

        let todos = client.query(&GetAllTodos).unwrap();
        assert_eq!(1, todos.len());
        assert_eq!(TODO_TEXT, todos[0].1);

        client.execute(&DropTodos).unwrap();
    }

    #[test]
    fn end_to_end_file() {
        const TODO_TEXT: &str = "get things done, please!";

        let mut client = Client::open("./foobar").unwrap();

        client.execute(&CreateTodos).unwrap();

        client.execute(&InsertTodo(TODO_TEXT)).unwrap();

        let todos = client.query(&GetAllTodos).unwrap();
        assert_eq!(1, todos.len());
        assert_eq!(TODO_TEXT, todos[0].1);

        client.execute(&DropTodos).unwrap();
    }
}
