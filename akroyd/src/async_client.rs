//! An asynchronous, pipelined, PostgreSQL client.

use crate::*;

/// An asynchronous PostgreSQL client.
pub struct Client {
    client: tokio_postgres::Client,
    statements: StatementCache,
}

/// A convenience function which parses a connection string and connects to the database.
///
/// See the documentation for `tokio_postgres::Config` for details on the connection string format.
pub async fn connect<T>(
    config: &str,
    tls: T,
) -> Result<
    (
        Client,
        tokio_postgres::Connection<tokio_postgres::Socket, T::Stream>,
    ),
    tokio_postgres::Error,
>
where
    T: tokio_postgres::tls::MakeTlsConnect<tokio_postgres::Socket>,
{
    let (client, connection) = tokio_postgres::connect(config, tls).await?;
    let client = Client::new(client);
    Ok((client, connection))
}

impl From<tokio_postgres::Client> for Client {
    fn from(client: tokio_postgres::Client) -> Self {
        Self::new(client)
    }
}

impl AsRef<tokio_postgres::Client> for Client {
    fn as_ref(&self) -> &tokio_postgres::Client {
        &self.client
    }
}

impl AsMut<tokio_postgres::Client> for Client {
    fn as_mut(&mut self) -> &mut tokio_postgres::Client {
        &mut self.client
    }
}

impl Client {
    /// Create a new `Client` from a `tokio_postgres::Client`.
    pub fn new(client: tokio_postgres::Client) -> Self {
        let statements = StatementCache::new();
        Client { client, statements }
    }

    fn statement_key<Q: Statement>() -> StatementKey {
        Q::TEXT.to_string()
    }

    async fn find_or_prepare<Q: Statement>(
        &mut self,
    ) -> Result<tokio_postgres::Statement, tokio_postgres::Error> {
        let key = Client::statement_key::<Q>();
        self.statements
            .ensure_async(key, || self.client.prepare(Q::TEXT))
            .await
    }

    /// Creates a new prepared statement.
    ///
    /// Everything required to prepare the statement is available on the
    /// type argument, so no runtime input is needed:
    ///
    /// ```no_run
    /// # async fn xmain() -> Result<(), tokio_postgres::Error> {
    /// # use akroyd::{Query, FromRow};
    /// # use akroyd::async_client::connect;
    /// # use tokio_postgres::NoTls;
    /// # #[derive(FromRow)]
    /// # pub struct Customer;
    /// #[derive(Query)]
    /// #[query(text = "SELECT id, first, last FROM customers WHERE first = $1", row(Customer))]
    /// pub struct GetCustomersByFirstName<'a>(&'a str);
    ///
    /// let (mut client, conn) = connect("host=localhost user=postgres", NoTls).await?;
    ///
    /// // Prepare the query in the database.
    /// client.prepare::<GetCustomersByFirstName>().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn prepare<Q: Statement>(&mut self) -> Result<(), tokio_postgres::Error> {
        self.find_or_prepare::<Q>().await?;
        Ok(())
    }

    /// Executes a statement, returning the resulting rows.
    ///
    /// We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # async fn xmain() -> Result<(), tokio_postgres::Error> {
    /// # use akroyd::{Query, FromRow};
    /// # use akroyd::async_client::connect;
    /// # use tokio_postgres::NoTls;
    /// # #[derive(FromRow)]
    /// # pub struct Customer {
    /// #   id: i32,
    /// #   first: String,
    /// #   last: String,
    /// # }
    /// #[derive(Query)]
    /// #[query(text = "SELECT id, first, last FROM customers WHERE first = $1", row(Customer))]
    /// pub struct GetCustomersByFirstName<'a>(&'a str);
    ///
    /// let (mut client, conn) = connect("host=localhost user=postgres", NoTls).await?;
    ///
    /// // Run the query and iterate over the results.
    /// for customer in client.query(&GetCustomersByFirstName("Sammy")).await? {
    ///     println!("Got customer {} {} with id {}", customer.first, customer.last, customer.id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn query<Q: Query>(
        &mut self,
        query: &Q,
    ) -> Result<Vec<Q::Row>, tokio_postgres::Error> {
        let stmt = self.find_or_prepare::<Q>().await?;
        self.client
            .query(&stmt, &query.to_row())
            .await?
            .into_iter()
            .map(FromRow::from_row)
            .collect()
    }

    /// Executes a statement which returns a single row, returning it.
    ///
    /// Returns an error if the query does not return exactly one row.  We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # async fn xmain() -> Result<(), tokio_postgres::Error> {
    /// # use akroyd::{QueryOne, FromRow};
    /// # use akroyd::async_client::connect;
    /// # use tokio_postgres::NoTls;
    /// # #[derive(FromRow)]
    /// # pub struct Customer {
    /// #   id: i32,
    /// #   first: String,
    /// #   last: String,
    /// # }
    /// #[derive(QueryOne)]
    /// #[query(text = "SELECT id, first, last FROM customers WHERE id = $1", row(Customer))]
    /// pub struct GetCustomerById(i32);
    ///
    /// let (mut client, conn) = connect("host=localhost user=postgres", NoTls).await?;
    ///
    /// // Run the query returning a single row.
    /// let customer = client.query_one(&GetCustomerById(42)).await?;
    /// println!("Got customer {} {} with id {}", customer.first, customer.last, customer.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn query_one<Q: QueryOne>(
        &mut self,
        query: &Q,
    ) -> Result<Q::Row, tokio_postgres::Error> {
        let stmt = self.find_or_prepare::<Q>().await?;
        FromRow::from_row(self.client.query_one(&stmt, &query.to_row()).await?)
    }

    /// Executes a statement which returns zero or one rows, returning it.
    ///
    /// Returns an error if the query returns more than one row.  We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # async fn xmain() -> Result<(), tokio_postgres::Error> {
    /// # use akroyd::{QueryOne, FromRow};
    /// # use akroyd::async_client::connect;
    /// # use tokio_postgres::NoTls;
    /// # #[derive(FromRow)]
    /// # pub struct Customer {
    /// #   id: i32,
    /// #   first: String,
    /// #   last: String,
    /// # }
    /// #[derive(QueryOne)]
    /// #[query(text = "SELECT id, first, last FROM customers WHERE id = $1", row(Customer))]
    /// pub struct GetCustomerById(i32);
    ///
    /// let (mut client, conn) = connect("host=localhost user=postgres", NoTls).await?;
    ///
    /// // Run the query, possibly returning a single row.
    /// if let Some(customer) = client.query_opt(&GetCustomerById(42)).await? {
    ///     println!("Got customer {} {} with id {}", customer.first, customer.last, customer.id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn query_opt<Q: QueryOne>(
        &mut self,
        query: &Q,
    ) -> Result<Option<Q::Row>, tokio_postgres::Error> {
        let stmt = self.find_or_prepare::<Q>().await?;
        self.client
            .query_opt(&stmt, &query.to_row())
            .await?
            .map(FromRow::from_row)
            .transpose()
    }

    /// Executes a statement, returning the number of rows modified.
    ///
    /// If the statement does not modify any rows (e.g. SELECT), 0 is returned.  We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # async fn xmain() -> Result<(), tokio_postgres::Error> {
    /// # use akroyd::{Statement};
    /// # use akroyd::async_client::connect;
    /// # use tokio_postgres::NoTls;
    /// #[derive(Statement)]
    /// #[query(text = "UPDATE customers SET first = $2, last = $3 WHERE id = $1")]
    /// pub struct UpdateCustomerName<'a>(i32, &'a str, &'a str);
    ///
    /// let (mut client, conn) = connect("host=localhost user=postgres", NoTls).await?;
    ///
    /// // Execute the statement, returning the number of rows modified.
    /// let rows_affected = client.execute(&UpdateCustomerName(42, "Anakin", "Skywalker")).await?;
    /// assert_eq!(rows_affected, 1);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn execute<Q: Statement>(&mut self, query: &Q) -> Result<u64, tokio_postgres::Error> {
        let stmt = self.find_or_prepare::<Q>().await?;
        self.client.execute(&stmt, &query.to_row()).await
    }

    /// Begins a new database transaction.
    ///
    /// The transaction will roll back by default - use the `commit` method to commit it.
    pub async fn transaction(&mut self) -> Result<Transaction, tokio_postgres::Error> {
        let txn = self.client.transaction().await?;
        let statements = &mut self.statements;
        Ok(Transaction { txn, statements })
    }
}

/// A representation of a PostgreSQL database transaction.
///
/// Transactions will implicitly roll back by default when dropped. Use the
/// `commit` method to commit the changes made in the transaction. Transactions
/// can be nested, with inner transactions implemented via savepoints.
pub struct Transaction<'a> {
    txn: tokio_postgres::Transaction<'a>,
    statements: &'a mut StatementCache,
}

impl<'a> AsRef<tokio_postgres::Transaction<'a>> for Transaction<'a> {
    fn as_ref(&self) -> &tokio_postgres::Transaction<'a> {
        &self.txn
    }
}

impl<'a> AsMut<tokio_postgres::Transaction<'a>> for Transaction<'a> {
    fn as_mut(&mut self) -> &mut tokio_postgres::Transaction<'a> {
        &mut self.txn
    }
}

impl<'a> Transaction<'a> {
    /// Consumes the transaction, committing all changes made within it.
    pub async fn commit(self) -> Result<(), tokio_postgres::Error> {
        self.txn.commit().await
    }

    /// Rolls the transaction back, discarding all changes made within it.
    ///
    /// This is equivalent to `Transaction`'s `Drop` implementation, but provides any error encountered to the caller.
    pub async fn rollback(self) -> Result<(), tokio_postgres::Error> {
        self.txn.rollback().await
    }

    async fn find_or_prepare<Q: Statement>(
        &mut self,
    ) -> Result<tokio_postgres::Statement, tokio_postgres::Error> {
        let key = Client::statement_key::<Q>();
        self.statements
            .ensure_async(key, || self.txn.prepare(Q::TEXT))
            .await
    }

    /// Creates a new prepared statement.
    ///
    /// Everything required to prepare the statement is available on the
    /// type argument, so no runtime input is needed:
    ///
    /// ```no_run
    /// # async fn xmain() -> Result<(), tokio_postgres::Error> {
    /// # use akroyd::{Query, FromRow};
    /// # use akroyd::async_client::connect;
    /// # use tokio_postgres::NoTls;
    /// # #[derive(FromRow)]
    /// # pub struct Customer;
    /// #[derive(Query)]
    /// #[query(text = "SELECT id, first, last FROM customers WHERE first = $1", row(Customer))]
    /// pub struct GetCustomersByFirstName<'a>(&'a str);
    ///
    /// let (mut client, conn) = connect("host=localhost user=postgres", NoTls).await?;
    /// let mut txn = client.transaction().await?;
    ///
    /// // Prepare the query in the database.
    /// txn.prepare::<GetCustomersByFirstName>().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn prepare<Q: Statement>(&mut self) -> Result<(), tokio_postgres::Error> {
        self.find_or_prepare::<Q>().await?;
        Ok(())
    }

    /// Executes a statement, returning the resulting rows.
    ///
    /// We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # async fn xmain() -> Result<(), tokio_postgres::Error> {
    /// # use akroyd::{Query, FromRow};
    /// # use akroyd::async_client::connect;
    /// # use tokio_postgres::NoTls;
    /// # #[derive(FromRow)]
    /// # pub struct Customer {
    /// #   id: i32,
    /// #   first: String,
    /// #   last: String,
    /// # }
    /// #[derive(Query)]
    /// #[query(text = "SELECT id, first, last FROM customers WHERE first = $1", row(Customer))]
    /// pub struct GetCustomersByFirstName<'a>(&'a str);
    ///
    /// let (mut client, conn) = connect("host=localhost user=postgres", NoTls).await?;
    /// let mut txn = client.transaction().await?;
    ///
    /// // Run the query and iterate over the results.
    /// for customer in txn.query(&GetCustomersByFirstName("Sammy")).await? {
    ///     println!("Got customer {} {} with id {}", customer.first, customer.last, customer.id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn query<Q: Query>(
        &mut self,
        query: &Q,
    ) -> Result<Vec<Q::Row>, tokio_postgres::Error> {
        let stmt = self.find_or_prepare::<Q>().await?;
        self.txn
            .query(&stmt, &query.to_row())
            .await?
            .into_iter()
            .map(FromRow::from_row)
            .collect()
    }

    /// Executes a statement which returns a single row, returning it.
    ///
    /// Returns an error if the query does not return exactly one row.  We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # async fn xmain() -> Result<(), tokio_postgres::Error> {
    /// # use akroyd::{QueryOne, FromRow};
    /// # use akroyd::async_client::connect;
    /// # use tokio_postgres::NoTls;
    /// # #[derive(FromRow)]
    /// # pub struct Customer {
    /// #   id: i32,
    /// #   first: String,
    /// #   last: String,
    /// # }
    /// #[derive(QueryOne)]
    /// #[query(text = "SELECT id, first, last FROM customers WHERE id = $1", row(Customer))]
    /// pub struct GetCustomerById(i32);
    ///
    /// let (mut client, conn) = connect("host=localhost user=postgres", NoTls).await?;
    /// let mut txn = client.transaction().await?;
    ///
    /// // Run the query returning a single row.
    /// let customer = txn.query_one(&GetCustomerById(42)).await?;
    /// println!("Got customer {} {} with id {}", customer.first, customer.last, customer.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn query_one<Q: QueryOne>(
        &mut self,
        query: &Q,
    ) -> Result<Q::Row, tokio_postgres::Error> {
        let stmt = self.find_or_prepare::<Q>().await?;
        FromRow::from_row(self.txn.query_one(&stmt, &query.to_row()).await?)
    }

    /// Executes a statement which returns zero or one rows, returning it.
    ///
    /// Returns an error if the query returns more than one row.  We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # async fn xmain() -> Result<(), tokio_postgres::Error> {
    /// # use akroyd::{QueryOne, FromRow};
    /// # use akroyd::async_client::connect;
    /// # use tokio_postgres::NoTls;
    /// # #[derive(FromRow)]
    /// # pub struct Customer {
    /// #   id: i32,
    /// #   first: String,
    /// #   last: String,
    /// # }
    /// #[derive(QueryOne)]
    /// #[query(text = "SELECT id, first, last FROM customers WHERE id = $1", row(Customer))]
    /// pub struct GetCustomerById(i32);
    ///
    /// let (mut client, conn) = connect("host=localhost user=postgres", NoTls).await?;
    /// let mut txn = client.transaction().await?;
    ///
    /// // Run the query, possibly returning a single row.
    /// if let Some(customer) = txn.query_opt(&GetCustomerById(42)).await? {
    ///     println!("Got customer {} {} with id {}", customer.first, customer.last, customer.id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn query_opt<Q: QueryOne>(
        &mut self,
        query: &Q,
    ) -> Result<Option<Q::Row>, tokio_postgres::Error> {
        let stmt = self.find_or_prepare::<Q>().await?;
        self.txn
            .query_opt(&stmt, &query.to_row())
            .await?
            .map(FromRow::from_row)
            .transpose()
    }

    /// Executes a statement, returning the number of rows modified.
    ///
    /// If the statement does not modify any rows (e.g. SELECT), 0 is returned.  We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # async fn xmain() -> Result<(), tokio_postgres::Error> {
    /// # use akroyd::{Statement};
    /// # use akroyd::async_client::connect;
    /// # use tokio_postgres::NoTls;
    /// #[derive(Statement)]
    /// #[query(text = "UPDATE customers SET first = $2, last = $3 WHERE id = $1")]
    /// pub struct UpdateCustomerName<'a>(i32, &'a str, &'a str);
    ///
    /// let (mut client, conn) = connect("host=localhost user=postgres", NoTls).await?;
    /// let mut txn = client.transaction().await?;
    ///
    /// // Execute the statement, returning the number of rows modified.
    /// let rows_affected = txn.execute(&UpdateCustomerName(42, "Anakin", "Skywalker")).await?;
    /// assert_eq!(rows_affected, 1);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn execute<Q: Statement>(&mut self, query: &Q) -> Result<u64, tokio_postgres::Error> {
        let stmt = self.find_or_prepare::<Q>().await?;
        self.txn.execute(&stmt, &query.to_row()).await
    }
}
