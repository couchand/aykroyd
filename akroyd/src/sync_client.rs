use crate::*;

/// A synchronous PostgreSQL client.
#[cfg(feature = "sync")]
pub struct Client {
    client: postgres::Client,
    statements: std::collections::HashMap<StatementKey, tokio_postgres::Statement>,
}

#[cfg(feature = "sync")]
impl Client {
    /// A convenience function which parses a configuration string into a `Config` and then connects to the database.
    ///
    /// See the documentation for `postgres::Config` for information about the connection syntax.
    ///
    /// ```no_run
    /// # fn main() -> Result<(), postgres::Error> {
    /// # use postgres::NoTls;
    /// // Connect to the database.
    /// let mut client = akroyd::Client::connect("host=localhost user=postgres", NoTls)?;
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
        let statements = std::collections::HashMap::new();
        Ok(Client { client, statements })
    }

    fn statement_key<Q: Statement>() -> StatementKey {
        Q::TEXT.to_string()
    }

    fn find_or_prepare<Q: Statement>(&mut self) -> Result<tokio_postgres::Statement, tokio_postgres::Error> {
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
    /// # use postgres::NoTls;
    /// # #[derive(FromRow)]
    /// # pub struct Customer;
    /// #[derive(Query)]
    /// #[query(text = "SELECT id, first, last FROM customers WHERE first = $1", row(Customer))]
    /// pub struct GetCustomersByFirstName<'a>(&'a str);
    ///
    /// let mut client = akroyd::Client::connect("host=localhost user=postgres", NoTls)?;
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
    /// let mut client = akroyd::Client::connect("host=localhost user=postgres", NoTls)?;
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
        Ok(self.client.query(&stmt, &query.to_row())?.into_iter().map(FromRow::from_row).collect::<Result<Vec<_>, _>>()?)
    }

    /// Executes a statement which returns a single row, returning it.
    ///
    /// Returns an error if the query does not return exactly one row.  We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # fn main() -> Result<(), postgres::Error> {
    /// # use akroyd::{QueryOne, FromRow};
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
    /// let mut client = akroyd::Client::connect("host=localhost user=postgres", NoTls)?;
    ///
    /// // Run the query returning a single row.
    /// let customer = client.query_one(&GetCustomerById(42))?;
    /// println!("Got customer {} {} with id {}", customer.first, customer.last, customer.id);
    /// # Ok(())
    /// # }
    /// ```
    pub fn query_one<Q: QueryOne>(&mut self, query: &Q) -> Result<Q::Row, tokio_postgres::Error> {
        let stmt = self.find_or_prepare::<Q>()?;
        Ok(FromRow::from_row(self.client.query_one(&stmt, &query.to_row())?)?)
    }

    /// Executes a statement which returns zero or one rows, returning it.
    ///
    /// Returns an error if the query returns more than one row.  We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # fn main() -> Result<(), postgres::Error> {
    /// # use akroyd::{QueryOne, FromRow};
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
    /// let mut client = akroyd::Client::connect("host=localhost user=postgres", NoTls)?;
    ///
    /// // Run the query, possibly returning a single row.
    /// if let Some(customer) = client.query_opt(&GetCustomerById(42))? {
    ///     println!("Got customer {} {} with id {}", customer.first, customer.last, customer.id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn query_opt<Q: QueryOne>(&mut self, query: &Q) -> Result<Option<Q::Row>, tokio_postgres::Error> {
        let stmt = self.find_or_prepare::<Q>()?;
        Ok(self.client.query_opt(&stmt, &query.to_row())?.map(FromRow::from_row).transpose()?)
    }

    /// Executes a statement, returning the number of rows modified.
    ///
    /// If the statement does not modify any rows (e.g. SELECT), 0 is returned.  We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # fn main() -> Result<(), postgres::Error> {
    /// # use akroyd::{Statement};
    /// # use postgres::NoTls;
    /// #[derive(Statement)]
    /// #[query(text = "UPDATE customers SET first = $2, last = $3 WHERE id = $1")]
    /// pub struct UpdateCustomerName<'a>(i32, &'a str, &'a str);
    ///
    /// let mut client = akroyd::Client::connect("host=localhost user=postgres", NoTls)?;
    ///
    /// // Execute the statement, returning the number of rows modified.
    /// let rows_affected = client.execute(&UpdateCustomerName(42, "Anakin", "Skywalker"))?;
    /// assert_eq!(rows_affected, 1);
    /// # Ok(())
    /// # }
    /// ```
    pub fn execute<Q: Statement>(&mut self, query: &Q) -> Result<u64, tokio_postgres::Error> {
        let stmt = self.find_or_prepare::<Q>()?;
        Ok(self.client.execute(&stmt, &query.to_row())?)
    }

    pub fn batch_execute(&mut self, statements: &str) -> Result<(), tokio_postgres::Error> {
        self.client.batch_execute(statements)
    }
}
