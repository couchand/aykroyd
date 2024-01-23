//! MySQL bindings.

use crate::client::{FromColumnIndexed, FromColumnNamed, ToParam};
use crate::query::StaticQueryText;
use crate::{error, FromRow, Query, QueryOne, Statement};

/// The type of errors from a `Client`.
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

/// A synchronous MySQL client.
#[derive(Debug)]
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
    /// Creates a new `Client` with the given options.
    ///
    /// ```no_run
    /// # use aykroyd::mysql::{Client, Error};
    /// # fn main() -> Result<(), Error> {
    /// let url = "mysql://user:password@locahost:3307/db_name";
    /// let mut client = Client::new(url)?;
    /// // ...
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # More Details
    ///
    /// See the docs for [`mysql::Conn::new()`] for more details.
    pub fn new<T, E>(opts: T) -> Result<Self, Error>
    where
        mysql::Opts: TryFrom<T, Error = E>,
        mysql::Error: From<E>,
    {
        mysql::Conn::new(opts).map(Client).map_err(Error::connect)
    }

    /// Creates and caches new prepared statement.
    ///
    /// Everything required to prepare the statement is available on the
    /// type argument, so no runtime input is needed:
    ///
    /// ```no_run
    /// # fn main() -> Result<(), aykroyd::mysql::Error> {
    /// # use aykroyd::{Query, FromRow};
    /// # use aykroyd::mysql::Client;
    /// # #[derive(FromRow)]
    /// # pub struct Customer;
    /// #[derive(Query)]
    /// #[aykroyd(row(Customer), text = "
    ///     SELECT id, first, last FROM customers WHERE first = ?
    /// ")]
    /// pub struct GetCustomersByFirstName<'a>(&'a str);
    ///
    /// let url = "mysql://user:password@locahost:3307/db_name";
    /// let mut client = Client::new(url)?;
    ///
    /// // Prepare the query in the database.
    /// client.prepare::<GetCustomersByFirstName>()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn prepare<S: StaticQueryText>(&mut self) -> Result<(), Error> {
        use mysql::prelude::Queryable;
        self.0.prep(S::QUERY_TEXT).map_err(Error::prepare)?;
        Ok(())
    }

    /// Executes a statement, returning the resulting rows.
    ///
    /// We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # fn main() -> Result<(), aykroyd::mysql::Error> {
    /// # use aykroyd::{Query, FromRow};
    /// # use aykroyd::mysql::Client;
    /// # #[derive(FromRow)]
    /// # pub struct Customer {
    /// #   id: i32,
    /// #   first: String,
    /// #   last: String,
    /// # }
    /// #[derive(Query)]
    /// #[aykroyd(row(Customer), text = "
    ///     SELECT id, first, last FROM customers WHERE first = ?
    /// ")]
    /// pub struct GetCustomersByFirstName<'a>(&'a str);
    ///
    /// let url = "mysql://user:password@locahost:3307/db_name";
    /// let mut client = Client::new(url)?;
    ///
    /// // Run the query and iterate over the results.
    /// for customer in client.query(&GetCustomersByFirstName("Sammy"))? {
    ///     println!("Got customer {} {} with id {}", customer.first, customer.last, customer.id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
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

        mysql::prelude::Queryable::exec_fold(
            self.as_mut(),
            &query,
            params,
            Ok(vec![]),
            |prior, row| match prior {
                Err(e) => Err(e),
                Ok(mut rows) => {
                    rows.push(FromRow::from_row(&row)?);
                    Ok(rows)
                }
            },
        )
        .map_err(Error::query)?
    }

    /// Executes a statement which returns a single row, returning it.
    ///
    /// Returns an error if the query does not return exactly one row.  We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # fn main() -> Result<(), aykroyd::mysql::Error> {
    /// # use aykroyd::{QueryOne, FromRow};
    /// # use aykroyd::mysql::Client;
    /// # #[derive(FromRow)]
    /// # pub struct Customer {
    /// #   id: i32,
    /// #   first: String,
    /// #   last: String,
    /// # }
    /// #[derive(QueryOne)]
    /// #[aykroyd(row(Customer), text = "
    ///     SELECT id, first, last FROM customers WHERE id = ?
    /// ")]
    /// pub struct GetCustomerById(i32);
    ///
    /// let url = "mysql://user:password@locahost:3307/db_name";
    /// let mut client = Client::new(url)?;
    ///
    /// // Run the query returning a single row.
    /// let customer = client.query_one(&GetCustomerById(42))?;
    /// println!("Got customer {} {} with id {}", customer.first, customer.last, customer.id);
    /// # Ok(())
    /// # }
    /// ```
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
            mysql::prelude::Queryable::exec_first(self.as_mut(), &query, params)
                .map_err(Error::query)?;

        row.ok_or_else(|| Error::query_str("query returned no rows", None))
            .and_then(|row| FromRow::from_row(&row))
    }

    /// Executes a statement which returns zero or one rows, returning it.
    ///
    /// Returns an error if the query returns more than one row.  We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # fn main() -> Result<(), aykroyd::mysql::Error> {
    /// # use aykroyd::{QueryOne, FromRow};
    /// # use aykroyd::mysql::Client;
    /// # #[derive(FromRow)]
    /// # pub struct Customer {
    /// #   id: i32,
    /// #   first: String,
    /// #   last: String,
    /// # }
    /// #[derive(QueryOne)]
    /// #[aykroyd(row(Customer), text = "
    ///     SELECT id, first, last FROM customers WHERE id = ?
    /// ")]
    /// pub struct GetCustomerById(i32);
    ///
    /// let url = "mysql://user:password@locahost:3307/db_name";
    /// let mut client = Client::new(url)?;
    ///
    /// // Run the query, possibly returning a single row.
    /// if let Some(customer) = client.query_opt(&GetCustomerById(42))? {
    ///     println!("Got customer {} {} with id {}", customer.first, customer.last, customer.id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
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
            mysql::prelude::Queryable::exec_first(self.as_mut(), &query, params)
                .map_err(Error::query)?;

        row.map(|row| FromRow::from_row(&row)).transpose()
    }

    /// Executes a statement, returning the number of rows modified.
    ///
    /// If the statement does not modify any rows (e.g. SELECT), 0 is returned.  We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # fn main() -> Result<(), aykroyd::mysql::Error> {
    /// # use aykroyd::{Statement};
    /// # use aykroyd::mysql::Client;
    /// #[derive(Statement)]
    /// #[aykroyd(text = "
    ///     UPDATE customers SET first = $2, last = $3 WHERE id = ?
    /// ")]
    /// pub struct UpdateCustomerName<'a>(i32, &'a str, &'a str);
    ///
    /// let url = "mysql://user:password@locahost:3307/db_name";
    /// let mut client = Client::new(url)?;
    ///
    /// // Execute the statement, returning the number of rows modified.
    /// let rows_affected = client.execute(&UpdateCustomerName(42, "Anakin", "Skywalker"))?;
    /// assert_eq!(rows_affected, 1);
    /// # Ok(())
    /// # }
    /// ```
    pub fn execute<S: Statement<Self>>(&mut self, statement: &S) -> Result<u64, Error> {
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

    /// Begins a new database transaction.
    ///
    /// The transaction will roll back by default - use the `commit` method to commit it.
    pub fn transaction(&mut self) -> Result<Transaction<'_>, Error> {
        Ok(Transaction(
            self.0
                .start_transaction(mysql::TxOpts::default())
                .map_err(Error::transaction)?,
        ))
    }
}

/// A synchronous MySQL transaction.
///
/// Transactions will implicitly roll back by default when dropped. Use the
/// `commit` method to commit the changes made in the transaction.
#[derive(Debug)]
pub struct Transaction<'a>(mysql::Transaction<'a>);

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
    /// # fn main() -> Result<(), aykroyd::mysql::Error> {
    /// # use aykroyd::{Query, FromRow};
    /// # use aykroyd::mysql::Client;
    /// # #[derive(FromRow)]
    /// # pub struct Customer;
    /// #[derive(Query)]
    /// #[aykroyd(row(Customer), text = "
    ///     SELECT id, first, last FROM customers WHERE first = ?
    /// ")]
    /// pub struct GetCustomersByFirstName<'a>(&'a str);
    ///
    /// let url = "mysql://user:password@locahost:3307/db_name";
    /// let mut client = Client::new(url)?;
    /// let mut txn = client.transaction()?;
    ///
    /// // Prepare the query in the database.
    /// txn.prepare::<GetCustomersByFirstName>()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn prepare<S: StaticQueryText>(&mut self) -> Result<(), Error> {
        use mysql::prelude::Queryable;
        self.0.prep(S::QUERY_TEXT).map_err(Error::prepare)?;
        Ok(())
    }

    /// Executes a statement, returning the resulting rows.
    ///
    /// We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # fn main() -> Result<(), aykroyd::mysql::Error> {
    /// # use aykroyd::{Query, FromRow};
    /// # use aykroyd::mysql::Client;
    /// # #[derive(FromRow)]
    /// # pub struct Customer {
    /// #   id: i32,
    /// #   first: String,
    /// #   last: String,
    /// # }
    /// #[derive(Query)]
    /// #[aykroyd(row(Customer), text = "
    ///     SELECT id, first, last FROM customers WHERE first = ?
    /// ")]
    /// pub struct GetCustomersByFirstName<'a>(&'a str);
    ///
    /// let url = "mysql://user:password@locahost:3307/db_name";
    /// let mut client = Client::new(url)?;
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
        use mysql::prelude::Queryable;

        let params = match query.to_params() {
            None => mysql::Params::Empty,
            Some(params) => mysql::Params::Positional(params),
        };
        let query = self.0.prep(query.query_text()).map_err(Error::prepare)?;

        mysql::prelude::Queryable::exec_fold(
            &mut self.0,
            &query,
            params,
            Ok(vec![]),
            |prior, row| match prior {
                Err(e) => Err(e),
                Ok(mut rows) => {
                    rows.push(FromRow::from_row(&row)?);
                    Ok(rows)
                }
            },
        )
        .map_err(Error::query)?
    }

    /// Executes a statement which returns a single row, returning it.
    ///
    /// Returns an error if the query does not return exactly one row.  We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # fn main() -> Result<(), aykroyd::mysql::Error> {
    /// # use aykroyd::{QueryOne, FromRow};
    /// # use aykroyd::mysql::Client;
    /// # #[derive(FromRow)]
    /// # pub struct Customer {
    /// #   id: i32,
    /// #   first: String,
    /// #   last: String,
    /// # }
    /// #[derive(QueryOne)]
    /// #[aykroyd(row(Customer), text = "
    ///     SELECT id, first, last FROM customers WHERE id = ?
    /// ")]
    /// pub struct GetCustomerById(i32);
    ///
    /// let url = "mysql://user:password@locahost:3307/db_name";
    /// let mut client = Client::new(url)?;
    /// let mut txn = client.transaction()?;
    ///
    /// // Run the query returning a single row.
    /// let customer = txn.query_one(&GetCustomerById(42))?;
    /// println!("Got customer {} {} with id {}", customer.first, customer.last, customer.id);
    /// # Ok(())
    /// # }
    /// ```
    pub fn query_one<Q: QueryOne<Client>>(&mut self, query: &Q) -> Result<Q::Row, Error> {
        use mysql::prelude::Queryable;

        let params = match query.to_params() {
            None => mysql::Params::Empty,
            Some(params) => mysql::Params::Positional(params),
        };
        let query = self.0.prep(query.query_text()).map_err(Error::prepare)?;

        let row: Option<mysql::Row> =
            mysql::prelude::Queryable::exec_first(&mut self.0, &query, params)
                .map_err(Error::query)?;

        row.ok_or_else(|| Error::query_str("query returned no rows", None))
            .and_then(|row| FromRow::from_row(&row))
    }

    /// Executes a statement which returns zero or one rows, returning it.
    ///
    /// Returns an error if the query returns more than one row.  We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # fn main() -> Result<(), aykroyd::mysql::Error> {
    /// # use aykroyd::{QueryOne, FromRow};
    /// # use aykroyd::mysql::Client;
    /// # #[derive(FromRow)]
    /// # pub struct Customer {
    /// #   id: i32,
    /// #   first: String,
    /// #   last: String,
    /// # }
    /// #[derive(QueryOne)]
    /// #[aykroyd(row(Customer), text = "
    ///     SELECT id, first, last FROM customers WHERE id = ?
    /// ")]
    /// pub struct GetCustomerById(i32);
    ///
    /// let url = "mysql://user:password@locahost:3307/db_name";
    /// let mut client = Client::new(url)?;
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
        use mysql::prelude::Queryable;

        let params = match query.to_params() {
            None => mysql::Params::Empty,
            Some(params) => mysql::Params::Positional(params),
        };
        let query = self.0.prep(query.query_text()).map_err(Error::prepare)?;

        let row: Option<mysql::Row> =
            mysql::prelude::Queryable::exec_first(&mut self.0, &query, params)
                .map_err(Error::query)?;

        row.map(|row| FromRow::from_row(&row)).transpose()
    }

    /// Executes a statement, returning the number of rows modified.
    ///
    /// If the statement does not modify any rows (e.g. SELECT), 0 is returned.  We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # fn main() -> Result<(), aykroyd::mysql::Error> {
    /// # use aykroyd::{Statement};
    /// # use aykroyd::mysql::Client;
    /// #[derive(Statement)]
    /// #[aykroyd(text = "
    ///     UPDATE customers SET first = $2, last = $3 WHERE id = ?
    /// ")]
    /// pub struct UpdateCustomerName<'a>(i32, &'a str, &'a str);
    ///
    /// let url = "mysql://user:password@locahost:3307/db_name";
    /// let mut client = Client::new(url)?;
    /// let mut txn = client.transaction()?;
    ///
    /// // Execute the statement, returning the number of rows modified.
    /// let rows_affected = txn.execute(&UpdateCustomerName(42, "Anakin", "Skywalker"))?;
    /// assert_eq!(rows_affected, 1);
    /// # Ok(())
    /// # }
    /// ```
    pub fn execute<S: Statement<Client>>(&mut self, statement: &S) -> Result<u64, Error> {
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
}

// TODO: not derive support
#[cfg(all(test, feature = "derive"))]
mod test {
    use super::*;

    #[derive(Statement)]
    #[aykroyd(text = "CREATE TABLE test_mysql (id SERIAL PRIMARY KEY, label TEXT NOT NULL)")]
    struct CreateTodos;

    #[derive(Statement)]
    #[aykroyd(text = "DROP TABLE IF EXISTS test_mysql")]
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

        let mut client =
            Client::new("mysql://aykroyd_test:aykroyd_test@localhost:3306/aykroyd_test").unwrap();

        client.execute(&DropTodos).unwrap();

        client.execute(&CreateTodos).unwrap();

        client.execute(&InsertTodo(TODO_TEXT)).unwrap();

        let todos = client.query(&GetAllTodos).unwrap();
        assert_eq!(1, todos.len());
        assert_eq!(TODO_TEXT, todos[0].1);

        client.execute(&DropTodos).unwrap();
    }
}
