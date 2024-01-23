#![allow(clippy::redundant_closure)]
//! Sqlite bindings.

use crate::client::{FromColumnIndexed, FromColumnNamed, ToParam};
use crate::query::StaticQueryText;
use crate::{error, FromRow, Query, QueryOne, Statement};

/// The type of errors from a `Client`.
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

/// A synchronous Sqlite client.
#[derive(Debug)]
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
    /// Open a new connection to a SQLite database. If a database does not exist
    /// at the path, one is created.
    ///
    /// ```rust,no_run
    /// # use aykroyd::rusqlite::{Client, Error};
    /// # fn open_my_db() -> Result<(), Error> {
    ///     let path = "./my_db.db3";
    ///     let db = Client::open(path)?;
    ///     // Use the database somehow...
    ///     println!("{}", db.as_ref().is_autocommit());
    ///     Ok(())
    /// # }
    /// ```
    ///
    /// # Failure
    ///
    /// Will return `Err` if `path` cannot be converted to a C-compatible string
    /// or if the underlying SQLite open call fails.
    ///
    /// # More Details
    ///
    /// See the docs for [`rusqlite::Connection::open()`] for more details.
    pub fn open<P: AsRef<std::path::Path>>(path: P) -> Result<Self, Error> {
        rusqlite::Connection::open(path)
            .map(Client)
            .map_err(Error::connect)
    }

    /// Open a new connection to a SQLite database.
    ///
    /// [Database Connection](http://www.sqlite.org/c3ref/open.html) for a description of valid
    /// flag combinations.
    ///
    /// # Failure
    ///
    /// Will return `Err` if `path` cannot be converted to a C-compatible
    /// string or if the underlying SQLite open call fails.
    ///
    /// # More Details
    ///
    /// See the docs for [`rusqlite::Connection::open_with_flags()`] for more details.
    pub fn open_with_flags<P: AsRef<std::path::Path>>(
        path: P,
        flags: rusqlite::OpenFlags,
    ) -> Result<Self, Error> {
        rusqlite::Connection::open_with_flags(path, flags)
            .map(Client)
            .map_err(Error::connect)
    }

    /// Open a new connection to a SQLite database using the specific flags and
    /// vfs name.
    ///
    /// [Database Connection](http://www.sqlite.org/c3ref/open.html) for a description of valid
    /// flag combinations.
    ///
    /// # Failure
    ///
    /// Will return `Err` if either `path` or `vfs` cannot be converted to a
    /// C-compatible string or if the underlying SQLite open call fails.
    ///
    /// # More Details
    ///
    /// See the docs for [`rusqlite::Connection::open_with_flags_and_vfs()`] for more details.
    pub fn open_with_flags_and_vfs<P: AsRef<std::path::Path>>(
        path: P,
        flags: rusqlite::OpenFlags,
        vfs: &str,
    ) -> Result<Self, Error> {
        rusqlite::Connection::open_with_flags_and_vfs(path, flags, vfs)
            .map(Client)
            .map_err(Error::connect)
    }

    /// Open a new connection to an in-memory SQLite database.
    ///
    /// # Failure
    ///
    /// Will return `Err` if the underlying SQLite open call fails.
    ///
    /// # More Details
    ///
    /// See the docs for [`rusqlite::Connection::open_in_memory()`] for more details.
    pub fn open_in_memory() -> Result<Self, Error> {
        rusqlite::Connection::open_in_memory()
            .map(Client)
            .map_err(Error::connect)
    }

    /// Open a new connection to an in-memory SQLite database.
    ///
    /// [Database Connection](http://www.sqlite.org/c3ref/open.html) for a description of valid
    /// flag combinations.
    ///
    /// # Failure
    ///
    /// Will return `Err` if the underlying SQLite open call fails.
    ///
    /// # More Details
    ///
    /// See the docs for [`rusqlite::Connection::open_in_memory_with_flags()`] for more details.
    pub fn open_in_memory_with_flags(flags: rusqlite::OpenFlags) -> Result<Self, Error> {
        rusqlite::Connection::open_in_memory_with_flags(flags)
            .map(Client)
            .map_err(Error::connect)
    }

    /// Open a new connection to an in-memory SQLite database using the specific
    /// flags and vfs name.
    ///
    /// [Database Connection](http://www.sqlite.org/c3ref/open.html) for a description of valid
    /// flag combinations.
    ///
    /// # Failure
    ///
    /// Will return `Err` if `vfs` cannot be converted to a C-compatible
    /// string or if the underlying SQLite open call fails.
    ///
    /// # More Details
    ///
    /// See the docs for [`rusqlite::Connection::open_in_memory_with_flags_and_vfs()`]
    /// for more details.
    pub fn open_in_memory_with_flags_and_vfs(
        flags: rusqlite::OpenFlags,
        vfs: &str,
    ) -> Result<Self, Error> {
        rusqlite::Connection::open_in_memory_with_flags_and_vfs(flags, vfs)
            .map(Client)
            .map_err(Error::connect)
    }

    /// Creates and caches new prepared statement.
    ///
    /// Everything required to prepare the statement is available on the
    /// type argument, so no runtime input is needed:
    ///
    /// ```no_run
    /// # fn main() -> Result<(), aykroyd::rusqlite::Error> {
    /// # use aykroyd::{Query, FromRow};
    /// # use aykroyd::rusqlite::Client;
    /// # #[derive(FromRow)]
    /// # pub struct Customer;
    /// #[derive(Query)]
    /// #[aykroyd(row(Customer), text = "
    ///     SELECT id, first, last FROM customers WHERE first = $1
    /// ")]
    /// pub struct GetCustomersByFirstName<'a>(&'a str);
    ///
    /// let mut client = Client::open("/path/to/database")?;
    ///
    /// // Prepare the query in the database.
    /// client.prepare::<GetCustomersByFirstName>()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn prepare<S: StaticQueryText>(&mut self) -> Result<(), Error> {
        self.as_mut()
            .prepare_cached(S::QUERY_TEXT)
            .map_err(Error::prepare)?;
        Ok(())
    }

    /// Executes a statement, returning the resulting rows.
    ///
    /// We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # fn main() -> Result<(), aykroyd::rusqlite::Error> {
    /// # use aykroyd::{Query, FromRow};
    /// # use aykroyd::rusqlite::Client;
    /// # #[derive(FromRow)]
    /// # pub struct Customer {
    /// #   id: i32,
    /// #   first: String,
    /// #   last: String,
    /// # }
    /// #[derive(Query)]
    /// #[aykroyd(row(Customer), text = "
    ///     SELECT id, first, last FROM customers WHERE first = $1
    /// ")]
    /// pub struct GetCustomersByFirstName<'a>(&'a str);
    ///
    /// let mut client = Client::open("/path/to/database")?;
    ///
    /// // Run the query and iterate over the results.
    /// for customer in client.query(&GetCustomersByFirstName("Sammy"))? {
    ///     println!("Got customer {} {} with id {}", customer.first, customer.last, customer.id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn query<Q: Query<Self>>(&mut self, query: &Q) -> Result<Vec<Q::Row>, Error> {
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

    /// Executes a statement which returns a single row, returning it.
    ///
    /// Returns an error if the query does not return exactly one row.  We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # fn main() -> Result<(), aykroyd::rusqlite::Error> {
    /// # use aykroyd::{QueryOne, FromRow};
    /// # use aykroyd::rusqlite::Client;
    /// # #[derive(FromRow)]
    /// # pub struct Customer {
    /// #   id: i32,
    /// #   first: String,
    /// #   last: String,
    /// # }
    /// #[derive(QueryOne)]
    /// #[aykroyd(row(Customer), text = "
    ///     SELECT id, first, last FROM customers WHERE id = $1
    /// ")]
    /// pub struct GetCustomerById(i32);
    ///
    /// let mut client = Client::open("/path/to/database")?;
    ///
    /// // Run the query returning a single row.
    /// let customer = client.query_one(&GetCustomerById(42))?;
    /// println!("Got customer {} {} with id {}", customer.first, customer.last, customer.id);
    /// # Ok(())
    /// # }
    /// ```
    pub fn query_one<Q: QueryOne<Self>>(&mut self, query: &Q) -> Result<Q::Row, Error> {
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

    /// Executes a statement which returns zero or one rows, returning it.
    ///
    /// Returns an error if the query returns more than one row.  We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # fn main() -> Result<(), aykroyd::rusqlite::Error> {
    /// # use aykroyd::{QueryOne, FromRow};
    /// # use aykroyd::rusqlite::Client;
    /// # #[derive(FromRow)]
    /// # pub struct Customer {
    /// #   id: i32,
    /// #   first: String,
    /// #   last: String,
    /// # }
    /// #[derive(QueryOne)]
    /// #[aykroyd(row(Customer), text = "
    ///     SELECT id, first, last FROM customers WHERE id = $1
    /// ")]
    /// pub struct GetCustomerById(i32);
    ///
    /// let mut client = Client::open("/path/to/database")?;
    ///
    /// // Run the query, possibly returning a single row.
    /// if let Some(customer) = client.query_opt(&GetCustomerById(42))? {
    ///     println!("Got customer {} {} with id {}", customer.first, customer.last, customer.id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn query_opt<Q: QueryOne<Self>>(&mut self, query: &Q) -> Result<Option<Q::Row>, Error> {
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

    /// Executes a statement, returning the number of rows modified.
    ///
    /// If the statement does not modify any rows (e.g. SELECT), 0 is returned.  We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # fn main() -> Result<(), aykroyd::rusqlite::Error> {
    /// # use aykroyd::{Statement};
    /// # use aykroyd::rusqlite::Client;
    /// #[derive(Statement)]
    /// #[aykroyd(text = "
    ///     UPDATE customers SET first = $2, last = $3 WHERE id = $1
    /// ")]
    /// pub struct UpdateCustomerName<'a>(i32, &'a str, &'a str);
    ///
    /// let mut client = Client::open("/path/to/database")?;
    ///
    /// // Execute the statement, returning the number of rows modified.
    /// let rows_affected = client.execute(&UpdateCustomerName(42, "Anakin", "Skywalker"))?;
    /// assert_eq!(rows_affected, 1);
    /// # Ok(())
    /// # }
    /// ```
    pub fn execute<S: Statement<Self>>(&mut self, statement: &S) -> Result<u64, Error> {
        let params = statement.to_params();
        let params: &[_] = params.as_ref().map(AsRef::as_ref).unwrap_or(&[][..]);

        let mut statement =
            rusqlite::Connection::prepare_cached(self.as_mut(), &statement.query_text())
                .map_err(Error::prepare)?;

        let rows_affected = statement.execute(params).map_err(Error::query)?;

        Ok(rows_affected.try_into().unwrap_or_default())
    }

    /// Begins a new database transaction.
    ///
    /// The transaction will roll back by default - use the `commit` method to commit it.
    pub fn transaction(&mut self) -> Result<Transaction<'_>, Error> {
        Ok(Transaction(
            self.0.transaction().map_err(Error::transaction)?,
        ))
    }
}

/// A synchronous Sqlite transaction.
///
/// Transactions will implicitly roll back by default when dropped. Use the
/// `commit` method to commit the changes made in the transaction.
#[derive(Debug)]
pub struct Transaction<'a>(rusqlite::Transaction<'a>);

impl<'a> Transaction<'a> {
    /// Consumes the transaction, committing all changes made within it.
    pub fn commit(self) -> Result<(), Error> {
        self.0.commit().map_err(Error::transaction)
    }

    /// Rolls the transaction back, discarding all changes made within it.
    ///
    /// This is equivalent to `Transaction`'s `Drop` implementation, but provides any error encountered to the caller.
    pub fn rollback(self) -> Result<(), Error> {
        self.0.rollback().map_err(Error::transaction)
    }

    /// Creates and caches new prepared statement.
    ///
    /// Everything required to prepare the statement is available on the
    /// type argument, so no runtime input is needed:
    ///
    /// ```no_run
    /// # fn main() -> Result<(), aykroyd::rusqlite::Error> {
    /// # use aykroyd::{Query, FromRow};
    /// # use aykroyd::rusqlite::Client;
    /// # #[derive(FromRow)]
    /// # pub struct Customer;
    /// #[derive(Query)]
    /// #[aykroyd(row(Customer), text = "
    ///     SELECT id, first, last FROM customers WHERE first = $1
    /// ")]
    /// pub struct GetCustomersByFirstName<'a>(&'a str);
    ///
    /// let mut client = Client::open("/path/to/database")?;
    /// let mut txn = client.transaction()?;
    ///
    /// // Prepare the query in the database.
    /// txn.prepare::<GetCustomersByFirstName>()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn prepare<S: StaticQueryText>(&mut self) -> Result<(), Error> {
        self.0
            .prepare_cached(S::QUERY_TEXT)
            .map_err(Error::prepare)?;
        Ok(())
    }

    /// Executes a statement, returning the resulting rows.
    ///
    /// We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # fn main() -> Result<(), aykroyd::rusqlite::Error> {
    /// # use aykroyd::{Query, FromRow};
    /// # use aykroyd::rusqlite::Client;
    /// # #[derive(FromRow)]
    /// # pub struct Customer {
    /// #   id: i32,
    /// #   first: String,
    /// #   last: String,
    /// # }
    /// #[derive(Query)]
    /// #[aykroyd(row(Customer), text = "
    ///     SELECT id, first, last FROM customers WHERE first = $1
    /// ")]
    /// pub struct GetCustomersByFirstName<'a>(&'a str);
    ///
    /// let mut client = Client::open("/path/to/database")?;
    /// let mut txn = client.transaction()?;
    ///
    /// // Run the query and iterate over the results.
    /// for customer in txn.query(&GetCustomersByFirstName("Sammy"))? {
    ///     println!("Got customer {} {} with id {}", customer.first, customer.last, customer.id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn query<Q: Query<Client>>(&mut self, query: &Q) -> Result<Vec<Q::Row>, Error> {
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

    /// Executes a statement which returns a single row, returning it.
    ///
    /// Returns an error if the query does not return exactly one row.  We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # fn main() -> Result<(), aykroyd::rusqlite::Error> {
    /// # use aykroyd::{QueryOne, FromRow};
    /// # use aykroyd::rusqlite::Client;
    /// # #[derive(FromRow)]
    /// # pub struct Customer {
    /// #   id: i32,
    /// #   first: String,
    /// #   last: String,
    /// # }
    /// #[derive(QueryOne)]
    /// #[aykroyd(row(Customer), text = "
    ///     SELECT id, first, last FROM customers WHERE id = $1
    /// ")]
    /// pub struct GetCustomerById(i32);
    ///
    /// let mut client = Client::open("/path/to/database")?;
    /// let mut txn = client.transaction()?;
    ///
    /// // Run the query returning a single row.
    /// let customer = txn.query_one(&GetCustomerById(42))?;
    /// println!("Got customer {} {} with id {}", customer.first, customer.last, customer.id);
    /// # Ok(())
    /// # }
    /// ```
    pub fn query_one<Q: QueryOne<Client>>(&mut self, query: &Q) -> Result<Q::Row, Error> {
        let params = query.to_params();
        let params: &[_] = params.as_ref().map(AsRef::as_ref).unwrap_or(&[][..]);

        let mut statement = rusqlite::Connection::prepare_cached(&self.0, &query.query_text())
            .map_err(Error::prepare)?;

        let mut rows = statement.query(params).map_err(Error::query)?;

        rows.next()
            .map_err(Error::query)?
            .ok_or_else(|| Error::query(rusqlite::Error::QueryReturnedNoRows))
            .and_then(|row| FromRow::from_row(row))
    }

    /// Executes a statement which returns zero or one rows, returning it.
    ///
    /// Returns an error if the query returns more than one row.  We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # fn main() -> Result<(), aykroyd::rusqlite::Error> {
    /// # use aykroyd::{QueryOne, FromRow};
    /// # use aykroyd::rusqlite::Client;
    /// # #[derive(FromRow)]
    /// # pub struct Customer {
    /// #   id: i32,
    /// #   first: String,
    /// #   last: String,
    /// # }
    /// #[derive(QueryOne)]
    /// #[aykroyd(row(Customer), text = "
    ///     SELECT id, first, last FROM customers WHERE id = $1
    /// ")]
    /// pub struct GetCustomerById(i32);
    ///
    /// let mut client = Client::open("/path/to/database")?;
    /// let mut txn = client.transaction()?;
    ///
    /// // Run the query, possibly returning a single row.
    /// if let Some(customer) = txn.query_opt(&GetCustomerById(42))? {
    ///     println!("Got customer {} {} with id {}", customer.first, customer.last, customer.id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn query_opt<Q: QueryOne<Client>>(&mut self, query: &Q) -> Result<Option<Q::Row>, Error> {
        let params = query.to_params();
        let params: &[_] = params.as_ref().map(AsRef::as_ref).unwrap_or(&[][..]);

        let mut statement = rusqlite::Connection::prepare_cached(&self.0, &query.query_text())
            .map_err(Error::prepare)?;

        let mut rows = statement.query(params).map_err(Error::query)?;

        rows.next()
            .map_err(Error::query)?
            .map(|row| FromRow::from_row(row))
            .transpose()
    }

    /// Executes a statement, returning the number of rows modified.
    ///
    /// If the statement does not modify any rows (e.g. SELECT), 0 is returned.  We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # fn main() -> Result<(), aykroyd::rusqlite::Error> {
    /// # use aykroyd::{Statement};
    /// # use aykroyd::rusqlite::Client;
    /// #[derive(Statement)]
    /// #[aykroyd(text = "
    ///     UPDATE customers SET first = $2, last = $3 WHERE id = $1
    /// ")]
    /// pub struct UpdateCustomerName<'a>(i32, &'a str, &'a str);
    ///
    /// let mut client = Client::open("/path/to/database")?;
    /// let mut txn = client.transaction()?;
    ///
    /// // Execute the statement, returning the number of rows modified.
    /// let rows_affected = txn.execute(&UpdateCustomerName(42, "Anakin", "Skywalker"))?;
    /// assert_eq!(rows_affected, 1);
    /// # Ok(())
    /// # }
    /// ```
    pub fn execute<S: Statement<Client>>(&mut self, statement: &S) -> Result<u64, Error> {
        let params = statement.to_params();
        let params: &[_] = params.as_ref().map(AsRef::as_ref).unwrap_or(&[][..]);

        let mut statement = rusqlite::Connection::prepare_cached(&self.0, &statement.query_text())
            .map_err(Error::prepare)?;

        let rows_affected = statement.execute(params).map_err(Error::query)?;

        Ok(rows_affected.try_into().unwrap_or_default())
    }
}

// TODO: not derive support
#[cfg(all(test, feature = "derive"))]
mod test {
    use super::*;

    #[derive(Statement)]
    #[aykroyd(
        text = "CREATE TABLE test_rusqlite (id INTEGER PRIMARY KEY AUTOINCREMENT, label TEXT NOT NULL)"
    )]
    struct CreateTodos;

    #[derive(Statement)]
    #[aykroyd(text = "DROP TABLE IF EXISTS test_rusqlite")]
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

        client.execute(&DropTodos).unwrap();

        client.execute(&CreateTodos).unwrap();

        client.execute(&InsertTodo(TODO_TEXT)).unwrap();

        let todos = client.query(&GetAllTodos).unwrap();
        assert_eq!(1, todos.len());
        assert_eq!(TODO_TEXT, todos[0].1);

        client.execute(&DropTodos).unwrap();
    }
}
