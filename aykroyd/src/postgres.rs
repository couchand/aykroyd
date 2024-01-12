//! A synchronous client for PostgreSQL.

use crate::client::{FromColumnIndexed, FromColumnNamed, ToParam};
use crate::query::StaticQueryText;
use crate::{error, FromRow, Query, QueryOne, Statement};

/// The type of errors from a `Client`.
pub type Error = error::Error<tokio_postgres::Error>;

impl<T> FromColumnIndexed<Client> for T
where
    T: tokio_postgres::types::FromSqlOwned,
{
    fn from_column(
        row: &tokio_postgres::Row,
        index: usize,
    ) -> Result<Self, Error> {
        row.try_get(index).map_err(Error::from_column)
    }
}

impl<T> FromColumnNamed<Client> for T
where
    T: tokio_postgres::types::FromSqlOwned,
{
    fn from_column(
        row: &tokio_postgres::Row,
        name: &str,
    ) -> Result<Self, Error> {
        row.try_get(name).map_err(Error::from_column)
    }
}

impl<T> ToParam<Client> for T
where
    T: tokio_postgres::types::ToSql + Sync,
{
    fn to_param(&self) -> &(dyn tokio_postgres::types::ToSql + Sync) {
        self
    }
}

/// A synchronous PostgreSQL client.
pub struct Client {
    client: postgres::Client,
    statements: std::collections::HashMap<String, tokio_postgres::Statement>,
}

impl AsMut<postgres::Client> for Client {
    fn as_mut(&mut self) -> &mut postgres::Client {
        &mut self.client
    }
}

impl crate::client::Client for Client {
    type Row<'a> = tokio_postgres::Row;
    type Param<'a> = &'a (dyn tokio_postgres::types::ToSql + Sync);
    type Error = tokio_postgres::Error;
}

impl AsRef<postgres::Client> for Client {
    fn as_ref(&self) -> &postgres::Client {
        &self.client
    }
}

impl From<postgres::Client> for Client {
    fn from(client: postgres::Client) -> Self {
        Self::new(client)
    }
}

impl Client {
    /// Create a new `Client` from a `postgres::Client`.
    pub fn new(client: postgres::Client) -> Self {
        let statements = std::collections::HashMap::new();
        Client { client, statements }
    }

    /// A convenience function which parses a configuration string into a `Config` and then connects to the database.
    ///
    /// See the documentation for `postgres::Config` for information about the connection syntax.
    ///
    /// ```no_run
    /// # fn main() -> Result<(), aykroyd::postgres::Error> {
    /// # use postgres::NoTls;
    /// # use aykroyd::postgres::Client;
    /// // Connect to the database.
    /// let mut client = Client::connect("host=localhost user=postgres", NoTls)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn connect<T>(params: &str, tls_mode: T) -> Result<Self, Error>
    where
        T: postgres::tls::MakeTlsConnect<postgres::Socket> + 'static + Send,
        T::TlsConnect: Send,
        T::Stream: Send,
        <T::TlsConnect as postgres::tls::TlsConnect<postgres::Socket>>::Future: Send,
    {
        let client = postgres::Client::connect(params, tls_mode)
            .map_err(Error::connect)?;
        Ok(Self::new(client))
    }

    fn prepare_internal<S: Into<String>>(
        &mut self,
        query_text: S,
    ) -> Result<postgres::Statement, Error> {
        match self.statements.entry(query_text.into()) {
            std::collections::hash_map::Entry::Occupied(entry) => Ok(entry.get().clone()),
            std::collections::hash_map::Entry::Vacant(entry) => {
                let statement = self.client.prepare(entry.key()).map_err(Error::prepare)?;
                Ok(entry.insert(statement).clone())
            }
        }
    }

    /// Creates and caches new prepared statement.
    ///
    /// Everything required to prepare the statement is available on the
    /// type argument, so no runtime input is needed:
    ///
    /// ```no_run
    /// # fn main() -> Result<(), aykroyd::postgres::Error> {
    /// # use aykroyd::{Query, FromRow};
    /// # use aykroyd::postgres::Client;
    /// # use postgres::NoTls;
    /// # #[derive(FromRow)]
    /// # pub struct Customer;
    /// #[derive(Query)]
    /// #[aykroyd(row(Customer), text = "
    ///     SELECT id, first, last FROM customers WHERE first = $1
    /// ")]
    /// pub struct GetCustomersByFirstName<'a>(&'a str);
    ///
    /// let mut client = Client::connect("host=localhost user=postgres", NoTls)?;
    ///
    /// // Prepare the query in the database.
    /// client.prepare::<GetCustomersByFirstName>()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn prepare<S: StaticQueryText>(&mut self) -> Result<(), Error> {
        self.prepare_internal(S::QUERY_TEXT)?;
        Ok(())
    }

    /// Executes a statement, returning the resulting rows.
    ///
    /// We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # fn main() -> Result<(), aykroyd::postgres::Error> {
    /// # use aykroyd::{Query, FromRow};
    /// # use aykroyd::postgres::Client;
    /// # use postgres::NoTls;
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
    /// let mut client = Client::connect("host=localhost user=postgres", NoTls)?;
    ///
    /// // Run the query and iterate over the results.
    /// for customer in client.query(&GetCustomersByFirstName("Sammy"))? {
    ///     println!("Got customer {} {} with id {}", customer.first, customer.last, customer.id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn query<Q: Query<Self>>(
        &mut self,
        query: &Q,
    ) -> Result<Vec<Q::Row>, Error> {
        let params = query.to_params();
        let params = params.as_ref().map(AsRef::as_ref).unwrap_or(&[][..]);
        let statement = self.prepare_internal(query.query_text())?;

        let rows = self
            .client
            .query(&statement, params)
            .map_err(Error::query)?;

        FromRow::from_rows(&rows)
    }

    /// Executes a statement which returns a single row, returning it.
    ///
    /// Returns an error if the query does not return exactly one row.  We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # fn main() -> Result<(), aykroyd::postgres::Error> {
    /// # use aykroyd::{QueryOne, FromRow};
    /// # use aykroyd::postgres::Client;
    /// # use postgres::NoTls;
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
    /// let mut client = Client::connect("host=localhost user=postgres", NoTls)?;
    ///
    /// // Run the query returning a single row.
    /// let customer = client.query_one(&GetCustomerById(42))?;
    /// println!("Got customer {} {} with id {}", customer.first, customer.last, customer.id);
    /// # Ok(())
    /// # }
    /// ```
    pub fn query_one<Q: QueryOne<Self>>(
        &mut self,
        query: &Q,
    ) -> Result<Q::Row, Error> {
        let params = query.to_params();
        let params = params.as_ref().map(AsRef::as_ref).unwrap_or(&[][..]);
        let statement = self.prepare_internal(query.query_text())?;

        let row = self
            .client
            .query_one(&statement, params)
            .map_err(Error::query)?;

        FromRow::from_row(&row)
    }

    /// Executes a statement which returns zero or one rows, returning it.
    ///
    /// Returns an error if the query returns more than one row.  We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # fn main() -> Result<(), aykroyd::postgres::Error> {
    /// # use aykroyd::{QueryOne, FromRow};
    /// # use aykroyd::postgres::Client;
    /// # use postgres::NoTls;
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
    /// let mut client = Client::connect("host=localhost user=postgres", NoTls)?;
    ///
    /// // Run the query, possibly returning a single row.
    /// if let Some(customer) = client.query_opt(&GetCustomerById(42))? {
    ///     println!("Got customer {} {} with id {}", customer.first, customer.last, customer.id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn query_opt<Q: QueryOne<Self>>(
        &mut self,
        query: &Q,
    ) -> Result<Option<Q::Row>, Error> {
        let params = query.to_params();
        let params = params.as_ref().map(AsRef::as_ref).unwrap_or(&[][..]);
        let statement = self.prepare_internal(query.query_text())?;

        let row = self
            .client
            .query_opt(&statement, params)
            .map_err(Error::query)?;

        row.map(|row| FromRow::from_row(&row)).transpose()
    }

    /// Executes a statement, returning the number of rows modified.
    ///
    /// If the statement does not modify any rows (e.g. SELECT), 0 is returned.  We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # fn main() -> Result<(), aykroyd::postgres::Error> {
    /// # use aykroyd::{Statement};
    /// # use aykroyd::postgres::Client;
    /// # use postgres::NoTls;
    /// #[derive(Statement)]
    /// #[aykroyd(text = "
    ///     UPDATE customers SET first = $2, last = $3 WHERE id = $1
    /// ")]
    /// pub struct UpdateCustomerName<'a>(i32, &'a str, &'a str);
    ///
    /// let mut client = Client::connect("host=localhost user=postgres", NoTls)?;
    ///
    /// // Execute the statement, returning the number of rows modified.
    /// let rows_affected = client.execute(&UpdateCustomerName(42, "Anakin", "Skywalker"))?;
    /// assert_eq!(rows_affected, 1);
    /// # Ok(())
    /// # }
    /// ```
    pub fn execute<S: Statement<Self>>(
        &mut self,
        statement: &S,
    ) -> Result<u64, Error> {
        let params = statement.to_params();
        let params = params.as_ref().map(AsRef::as_ref).unwrap_or(&[][..]);
        let statement = self.prepare_internal(statement.query_text())?;

        let rows_affected = self
            .client
            .execute(&statement, params)
            .map_err(Error::query)?;

        Ok(rows_affected)
    }

    /// Begins a new database transaction.
    ///
    /// The transaction will roll back by default - use the `commit` method to commit it.
    pub fn transaction(&mut self) -> Result<Transaction, Error> {
        Ok(Transaction {
            txn: self.client.transaction().map_err(Error::transaction)?,
            statements: &mut self.statements,
        })
    }
}

/// A synchronous PostgreSQL transaction.
///
/// Transactions will implicitly roll back by default when dropped. Use the
/// `commit` method to commit the changes made in the transaction.
pub struct Transaction<'a> {
    txn: postgres::Transaction<'a>,
    statements: &'a mut std::collections::HashMap<String, tokio_postgres::Statement>,
}

impl<'a> Transaction<'a> {
    fn prepare_internal<S: Into<String>>(
        &mut self,
        query_text: S,
    ) -> Result<tokio_postgres::Statement, Error> {
        match self.statements.entry(query_text.into()) {
            std::collections::hash_map::Entry::Occupied(entry) => Ok(entry.get().clone()),
            std::collections::hash_map::Entry::Vacant(entry) => {
                let statement = self.txn.prepare(entry.key()).map_err(Error::prepare)?;
                Ok(entry.insert(statement).clone())
            }
        }
    }

    /// Consumes the transaction, committing all changes made within it.
    pub fn commit(self) -> Result<(), Error> {
        self.txn.commit().map_err(Error::transaction)
    }

    /// Rolls the transaction back, discarding all changes made within it.
    ///
    /// This is equivalent to `Transaction`'s `Drop` implementation, but provides any error encountered to the caller.
    pub fn rollback(self) -> Result<(), Error> {
        self.txn.rollback().map_err(Error::transaction)
    }

    /// Creates a new prepared statement.
    ///
    /// Everything required to prepare the statement is available on the
    /// type argument, so no runtime input is needed:
    ///
    /// ```no_run
    /// # fn main() -> Result<(), aykroyd::postgres::Error> {
    /// # use aykroyd::{Query, FromRow};
    /// # use aykroyd::postgres::Client;
    /// # use postgres::NoTls;
    /// # #[derive(FromRow)]
    /// # pub struct Customer;
    /// #[derive(Query)]
    /// #[aykroyd(row(Customer), text = "
    ///     SELECT id, first, last FROM customers WHERE first = $1
    /// ")]
    /// pub struct GetCustomersByFirstName<'a>(&'a str);
    ///
    /// let mut client = Client::connect("host=localhost user=postgres", NoTls)?;
    /// let mut txn = client.transaction()?;
    ///
    /// // Prepare the query in the database.
    /// txn.prepare::<GetCustomersByFirstName>()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn prepare<S: StaticQueryText>(&mut self) -> Result<(), Error> {
        self.prepare_internal(S::QUERY_TEXT)?;
        Ok(())
    }

    /// Executes a statement, returning the resulting rows.
    ///
    /// We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # fn main() -> Result<(), aykroyd::postgres::Error> {
    /// # use aykroyd::{Query, FromRow};
    /// # use aykroyd::postgres::Client;
    /// # use postgres::NoTls;
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
    /// let mut client = Client::connect("host=localhost user=postgres", NoTls)?;
    /// let mut txn = client.transaction()?;
    ///
    /// // Run the query and iterate over the results.
    /// for customer in txn.query(&GetCustomersByFirstName("Sammy"))? {
    ///     println!("Got customer {} {} with id {}", customer.first, customer.last, customer.id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn query<Q: Query<Client>>(
        &mut self,
        query: &Q,
    ) -> Result<Vec<Q::Row>, Error> {
        let params = query.to_params();
        let params = params.as_ref().map(AsRef::as_ref).unwrap_or(&[][..]);
        let statement = self.prepare_internal(query.query_text())?;

        let rows = self.txn.query(&statement, params).map_err(Error::query)?;

        FromRow::from_rows(&rows)
    }

    /// Executes a statement which returns a single row, returning it.
    ///
    /// Returns an error if the query does not return exactly one row.  We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # fn main() -> Result<(), aykroyd::postgres::Error> {
    /// # use aykroyd::{QueryOne, FromRow};
    /// # use aykroyd::postgres::Client;
    /// # use postgres::NoTls;
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
    /// let mut client = Client::connect("host=localhost user=postgres", NoTls)?;
    /// let mut txn = client.transaction()?;
    ///
    /// // Run the query returning a single row.
    /// let customer = txn.query_one(&GetCustomerById(42))?;
    /// println!("Got customer {} {} with id {}", customer.first, customer.last, customer.id);
    /// # Ok(())
    /// # }
    /// ```
    pub fn query_one<Q: QueryOne<Client>>(
        &mut self,
        query: &Q,
    ) -> Result<Q::Row, Error> {
        let params = query.to_params();
        let params = params.as_ref().map(AsRef::as_ref).unwrap_or(&[][..]);
        let statement = self.prepare_internal(query.query_text())?;

        let row = self.txn.query_one(&statement, params).map_err(Error::query)?;

        FromRow::from_row(&row)
    }

    /// Executes a statement which returns zero or one rows, returning it.
    ///
    /// Returns an error if the query returns more than one row.  We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # fn main() -> Result<(), aykroyd::postgres::Error> {
    /// # use aykroyd::{QueryOne, FromRow};
    /// # use aykroyd::postgres::Client;
    /// # use postgres::NoTls;
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
    /// let mut client = Client::connect("host=localhost user=postgres", NoTls)?;
    /// let mut txn = client.transaction()?;
    ///
    /// // Run the query, possibly returning a single row.
    /// if let Some(customer) = txn.query_opt(&GetCustomerById(42))? {
    ///     println!("Got customer {} {} with id {}", customer.first, customer.last, customer.id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn query_opt<Q: QueryOne<Client>>(
        &mut self,
        query: &Q,
    ) -> Result<Option<Q::Row>, Error> {
        let params = query.to_params();
        let params = params.as_ref().map(AsRef::as_ref).unwrap_or(&[][..]);
        let statement = self.prepare_internal(query.query_text())?;

        let row = self
            .txn
            .query_opt(&statement, params)
            .map_err(Error::query)?;

        row.map(|row| FromRow::from_row(&row)).transpose()
    }

    /// Executes a statement, returning the number of rows modified.
    ///
    /// If the statement does not modify any rows (e.g. SELECT), 0 is returned.  We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # fn main() -> Result<(), aykroyd::postgres::Error> {
    /// # use aykroyd::{Statement};
    /// # use aykroyd::postgres::Client;
    /// # use postgres::NoTls;
    /// #[derive(Statement)]
    /// #[aykroyd(text = "
    ///     UPDATE customers SET first = $2, last = $3 WHERE id = $1
    /// ")]
    /// pub struct UpdateCustomerName<'a>(i32, &'a str, &'a str);
    ///
    /// let mut client = Client::connect("host=localhost user=postgres", NoTls)?;
    /// let mut txn = client.transaction()?;
    ///
    /// // Execute the statement, returning the number of rows modified.
    /// let rows_affected = txn.execute(&UpdateCustomerName(42, "Anakin", "Skywalker"))?;
    /// assert_eq!(rows_affected, 1);
    /// # Ok(())
    /// # }
    /// ```
    pub fn execute<S: Statement<Client>>(
        &mut self,
        statement: &S,
    ) -> Result<u64, Error> {
        let params = statement.to_params();
        let params = params.as_ref().map(AsRef::as_ref).unwrap_or(&[][..]);
        let statement = self.prepare_internal(statement.query_text())?;

        let rows_affected = self
            .txn
            .execute(&statement, params)
            .map_err(Error::query)?;

        Ok(rows_affected)
    }
}

// TODO: not derive support
#[cfg(all(test, feature ="derive"))]
mod test {
    use super::*;

    use postgres::NoTls;

    #[derive(Statement)]
    #[aykroyd(text = "CREATE TABLE test_postgres (id SERIAL PRIMARY KEY, label TEXT NOT NULL)")]
    struct CreateTodos;

    #[derive(Statement)]
    #[aykroyd(text = "DROP TABLE test_postgres")]
    struct DropTodos;

    #[derive(Statement)]
    #[aykroyd(text = "INSERT INTO test_postgres (label) VALUES ($1)")]
    struct InsertTodo<'a>(&'a str);

    #[derive(Query)]
    #[aykroyd(row((i32, String)), text = "SELECT id, label FROM test_postgres")]
    struct GetAllTodos;

    #[test]
    fn end_to_end() {
        const TODO_TEXT: &str = "get things done, please!";

        let mut client = Client::connect(
            "host=localhost user=aykroyd_test password=aykroyd_test",
            NoTls,
        ).unwrap();

        client.execute(&CreateTodos).unwrap();

        client.execute(&InsertTodo(TODO_TEXT)).unwrap();

        let todos = client.query(&GetAllTodos).unwrap();
        assert_eq!(1, todos.len());
        assert_eq!(TODO_TEXT, todos[0].1);

        client.execute(&DropTodos).unwrap();
    }
}
