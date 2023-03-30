use crate::*;

#[derive(Clone)]
struct StatementCache(std::rc::Rc<std::cell::RefCell<std::collections::HashMap<StatementKey, tokio_postgres::Statement>>>);

impl StatementCache {
    fn new() -> Self {
        StatementCache(std::rc::Rc::new(std::cell::RefCell::new(std::collections::HashMap::new())))
    }

    fn get(&self, key: &StatementKey) -> Option<tokio_postgres::Statement> {
        self.0.borrow().get(key).cloned()
    }

    fn insert(&self, key: StatementKey, statement: tokio_postgres::Statement) {
        self.0.borrow_mut().insert(key, statement);
    }
}

/// A synchronous PostgreSQL client.
pub struct Client {
    client: postgres::Client,
    statements: StatementCache,
}

impl From<postgres::Client> for Client {
    fn from(client: postgres::Client) -> Self {
        Self::new(client)
    }
}

impl AsRef<postgres::Client> for Client {
    fn as_ref(&self) -> &postgres::Client {
        &self.client
    }
}

impl AsMut<postgres::Client> for Client {
    fn as_mut(&mut self) -> &mut postgres::Client {
        &mut self.client
    }
}

impl Client {
    /// Create a new `Client` from a `postgres::Client`.
    pub fn new(client: postgres::Client) -> Self {
        let statements = StatementCache::new();
        Client { client, statements }
    }

    /// A convenience function which parses a configuration string into a `Config` and then connects to the database.
    ///
    /// See the documentation for `postgres::Config` for information about the connection syntax.
    ///
    /// ```no_run
    /// # fn main() -> Result<(), postgres::Error> {
    /// # use postgres::NoTls;
    /// # use akroyd::sync_client::Client;
    /// // Connect to the database.
    /// let mut client = Client::connect("host=localhost user=postgres", NoTls)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn connect<T>(params: &str, tls_mode: T) -> Result<Self, tokio_postgres::Error>
    where
        T: postgres::tls::MakeTlsConnect<postgres::Socket> + 'static + Send,
        T::TlsConnect: Send,
        T::Stream: Send,
        <T::TlsConnect as postgres::tls::TlsConnect<postgres::Socket>>::Future: Send,
    {
        let client = postgres::Client::connect(params, tls_mode)?;
        Ok(Self::new(client))
    }

    fn statement_key<Q: Statement>() -> StatementKey {
        Q::TEXT.to_string()
    }

    fn find_or_prepare<Q: Statement>(
        &mut self,
    ) -> Result<tokio_postgres::Statement, tokio_postgres::Error> {
        let key = Client::statement_key::<Q>();

        if self.statements.get(&key).is_none() {
            let key = key.clone();
            let prepared = self.client.prepare(Q::TEXT)?;
            self.statements.insert(key, prepared);
        }

        Ok(self.statements.get(&key).unwrap().clone())
    }

    /// Creates a new prepared statement.
    ///
    /// Everything required to prepare the statement is available on the
    /// type argument, so no runtime input is needed:
    ///
    /// ```no_run
    /// # fn main() -> Result<(), postgres::Error> {
    /// # use akroyd::{Query, FromRow};
    /// # use akroyd::sync_client::Client;
    /// # use postgres::NoTls;
    /// # #[derive(FromRow)]
    /// # pub struct Customer;
    /// #[derive(Query)]
    /// #[query(text = "SELECT id, first, last FROM customers WHERE first = $1", row(Customer))]
    /// pub struct GetCustomersByFirstName<'a>(&'a str);
    ///
    /// let mut client = Client::connect("host=localhost user=postgres", NoTls)?;
    ///
    /// // Prepare the query in the database.
    /// client.prepare::<GetCustomersByFirstName>()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn prepare<Q: Statement>(&mut self) -> Result<(), tokio_postgres::Error> {
        self.find_or_prepare::<Q>()?;
        Ok(())
    }

    /// Executes a statement, returning the resulting rows.
    ///
    /// We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # fn main() -> Result<(), postgres::Error> {
    /// # use akroyd::{Query, FromRow};
    /// # use akroyd::sync_client::Client;
    /// # use postgres::NoTls;
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
    /// let mut client = Client::connect("host=localhost user=postgres", NoTls)?;
    ///
    /// // Run the query and iterate over the results.
    /// for customer in client.query(&GetCustomersByFirstName("Sammy"))? {
    ///     println!("Got customer {} {} with id {}", customer.first, customer.last, customer.id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn query<Q: Query>(&mut self, query: &Q) -> Result<Vec<Q::Row>, tokio_postgres::Error> {
        let stmt = self.find_or_prepare::<Q>()?;
        self.client
            .query(&stmt, &query.to_row())?
            .into_iter()
            .map(FromRow::from_row)
            .collect()
    }

    /// Executes a statement which returns a single row, returning it.
    ///
    /// Returns an error if the query does not return exactly one row.  We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # fn main() -> Result<(), postgres::Error> {
    /// # use akroyd::{QueryOne, FromRow};
    /// # use akroyd::sync_client::Client;
    /// # use postgres::NoTls;
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
    /// let mut client = Client::connect("host=localhost user=postgres", NoTls)?;
    ///
    /// // Run the query returning a single row.
    /// let customer = client.query_one(&GetCustomerById(42))?;
    /// println!("Got customer {} {} with id {}", customer.first, customer.last, customer.id);
    /// # Ok(())
    /// # }
    /// ```
    pub fn query_one<Q: QueryOne>(&mut self, query: &Q) -> Result<Q::Row, tokio_postgres::Error> {
        let stmt = self.find_or_prepare::<Q>()?;
        FromRow::from_row(self.client.query_one(&stmt, &query.to_row())?)
    }

    /// Executes a statement which returns zero or one rows, returning it.
    ///
    /// Returns an error if the query returns more than one row.  We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # fn main() -> Result<(), postgres::Error> {
    /// # use akroyd::{QueryOne, FromRow};
    /// # use akroyd::sync_client::Client;
    /// # use postgres::NoTls;
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
    /// let mut client = Client::connect("host=localhost user=postgres", NoTls)?;
    ///
    /// // Run the query, possibly returning a single row.
    /// if let Some(customer) = client.query_opt(&GetCustomerById(42))? {
    ///     println!("Got customer {} {} with id {}", customer.first, customer.last, customer.id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn query_opt<Q: QueryOne>(
        &mut self,
        query: &Q,
    ) -> Result<Option<Q::Row>, tokio_postgres::Error> {
        let stmt = self.find_or_prepare::<Q>()?;
        self.client
            .query_opt(&stmt, &query.to_row())?
            .map(FromRow::from_row)
            .transpose()
    }

    /// Executes a statement, returning the number of rows modified.
    ///
    /// If the statement does not modify any rows (e.g. SELECT), 0 is returned.  We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # fn main() -> Result<(), postgres::Error> {
    /// # use akroyd::{Statement};
    /// # use akroyd::sync_client::Client;
    /// # use postgres::NoTls;
    /// #[derive(Statement)]
    /// #[query(text = "UPDATE customers SET first = $2, last = $3 WHERE id = $1")]
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
    pub fn execute<Q: Statement>(&mut self, query: &Q) -> Result<u64, tokio_postgres::Error> {
        let stmt = self.find_or_prepare::<Q>()?;
        self.client.execute(&stmt, &query.to_row())
    }

    /// Begins a new database transaction.
    ///
    /// The transaction will roll back by default - use the `commit` method to commit it.
    pub fn transaction(&mut self) -> Result<Transaction, tokio_postgres::Error> {
        let txn = self.client.transaction()?;
        let statements = self.statements.clone();
        Ok(Transaction { txn, statements })
    }
}

/// A representation of a PostgreSQL database transaction.
///
/// Transactions will implicitly roll back by default when dropped. Use the
/// `commit` method to commit the changes made in the transaction. Transactions
/// can be nested, with inner transactions implemented via savepoints.
pub struct Transaction<'a> {
    txn: postgres::Transaction<'a>,
    statements: StatementCache,
}

impl<'a> AsRef<postgres::Transaction<'a>> for Transaction<'a> {
    fn as_ref(&self) -> &postgres::Transaction<'a> {
        &self.txn
    }
}

impl<'a> AsMut<postgres::Transaction<'a>> for Transaction<'a> {
    fn as_mut(&mut self) -> &mut postgres::Transaction<'a> {
        &mut self.txn
    }
}

impl<'a> Transaction<'a> {
    /// Consumes the transaction, committing all changes made within it.
    pub fn commit(self) -> Result<(), tokio_postgres::Error> {
        self.txn.commit()
    }

    fn find_or_prepare<Q: Statement>(
        &mut self,
    ) -> Result<tokio_postgres::Statement, tokio_postgres::Error> {
        let key = Client::statement_key::<Q>();

        if self.statements.get(&key).is_none() {
            let key = key.clone();
            let prepared = self.txn.prepare(Q::TEXT)?;
            self.statements.insert(key, prepared);
        }

        Ok(self.statements.get(&key).unwrap().clone())
    }

    /// Creates a new prepared statement.
    ///
    /// Everything required to prepare the statement is available on the
    /// type argument, so no runtime input is needed:
    ///
    /// ```no_run
    /// # fn main() -> Result<(), postgres::Error> {
    /// # use akroyd::{Query, FromRow};
    /// # use akroyd::sync_client::Client;
    /// # use postgres::NoTls;
    /// # #[derive(FromRow)]
    /// # pub struct Customer;
    /// #[derive(Query)]
    /// #[query(text = "SELECT id, first, last FROM customers WHERE first = $1", row(Customer))]
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
    pub fn prepare<Q: Statement>(&mut self) -> Result<(), tokio_postgres::Error> {
        self.find_or_prepare::<Q>()?;
        Ok(())
    }

    /// Executes a statement, returning the resulting rows.
    ///
    /// We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # fn main() -> Result<(), postgres::Error> {
    /// # use akroyd::{Query, FromRow};
    /// # use akroyd::sync_client::Client;
    /// # use postgres::NoTls;
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
    pub fn query<Q: Query>(&mut self, query: &Q) -> Result<Vec<Q::Row>, tokio_postgres::Error> {
        let stmt = self.find_or_prepare::<Q>()?;
        self.txn
            .query(&stmt, &query.to_row())?
            .into_iter()
            .map(FromRow::from_row)
            .collect()
    }

    /// Executes a statement which returns a single row, returning it.
    ///
    /// Returns an error if the query does not return exactly one row.  We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # fn main() -> Result<(), postgres::Error> {
    /// # use akroyd::{QueryOne, FromRow};
    /// # use akroyd::sync_client::Client;
    /// # use postgres::NoTls;
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
    /// let mut client = Client::connect("host=localhost user=postgres", NoTls)?;
    /// let mut txn = client.transaction()?;
    ///
    /// // Run the query returning a single row.
    /// let customer = txn.query_one(&GetCustomerById(42))?;
    /// println!("Got customer {} {} with id {}", customer.first, customer.last, customer.id);
    /// # Ok(())
    /// # }
    /// ```
    pub fn query_one<Q: QueryOne>(&mut self, query: &Q) -> Result<Q::Row, tokio_postgres::Error> {
        let stmt = self.find_or_prepare::<Q>()?;
        FromRow::from_row(self.txn.query_one(&stmt, &query.to_row())?)
    }

    /// Executes a statement which returns zero or one rows, returning it.
    ///
    /// Returns an error if the query returns more than one row.  We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # fn main() -> Result<(), postgres::Error> {
    /// # use akroyd::{QueryOne, FromRow};
    /// # use akroyd::sync_client::Client;
    /// # use postgres::NoTls;
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
    pub fn query_opt<Q: QueryOne>(
        &mut self,
        query: &Q,
    ) -> Result<Option<Q::Row>, tokio_postgres::Error> {
        let stmt = self.find_or_prepare::<Q>()?;
        self.txn
            .query_opt(&stmt, &query.to_row())?
            .map(FromRow::from_row)
            .transpose()
    }

    /// Executes a statement, returning the number of rows modified.
    ///
    /// If the statement does not modify any rows (e.g. SELECT), 0 is returned.  We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # fn main() -> Result<(), postgres::Error> {
    /// # use akroyd::{Statement};
    /// # use akroyd::sync_client::Client;
    /// # use postgres::NoTls;
    /// #[derive(Statement)]
    /// #[query(text = "UPDATE customers SET first = $2, last = $3 WHERE id = $1")]
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
    pub fn execute<Q: Statement>(&mut self, query: &Q) -> Result<u64, tokio_postgres::Error> {
        let stmt = self.find_or_prepare::<Q>()?;
        self.txn.execute(&stmt, &query.to_row())
    }
}
