//! An asynchronous, pipelined, PostgreSQL client.

use crate::client::{FromColumnIndexed, FromColumnNamed, ToParam};
use crate::query::StaticQueryText;
use crate::{error, FromRow, Query, QueryOne, Statement};

pub type Error = error::Error<tokio_postgres::Error>;

/// A convenience function which parses a connection string and connects to the database.
///
/// See the documentation for [`tokio_postgres::Config`] for details on the connection string format.
pub async fn connect<T>(
    config: &str,
    tls: T,
) -> Result<
    (
        Client,
        tokio_postgres::Connection<tokio_postgres::Socket, T::Stream>,
    ),
    Error,
>
where
    T: tokio_postgres::tls::MakeTlsConnect<tokio_postgres::Socket>,
{
    let (client, connection) = tokio_postgres::connect(config, tls)
        .await
        .map_err(Error::connect)?;
    Ok((client.into(), connection))
}

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

/// An asynchronous PostgreSQL client.
pub struct Client {
    client: tokio_postgres::Client,
    statements: std::collections::HashMap<String, tokio_postgres::Statement>,
}

impl crate::client::Client for Client {
    type Row<'a> = tokio_postgres::Row;
    type Param<'a> = &'a (dyn tokio_postgres::types::ToSql + Sync);
    type Error = tokio_postgres::Error;
}

impl AsMut<tokio_postgres::Client> for Client {
    fn as_mut(&mut self) -> &mut tokio_postgres::Client {
        &mut self.client
    }
}

impl AsRef<tokio_postgres::Client> for Client {
    fn as_ref(&self) -> &tokio_postgres::Client {
        &self.client
    }
}

impl From<tokio_postgres::Client> for Client {
    fn from(client: tokio_postgres::Client) -> Self {
        Self::new(client)
    }
}

impl Client {
    /// Create a new `Client` from a `tokio_postgres::Client`.
    pub fn new(client: tokio_postgres::Client) -> Self {
        let statements = std::collections::HashMap::new();
        Client { client, statements }
    }

    async fn prepare_internal<S: Into<String>>(
        &mut self,
        query_text: S,
    ) -> Result<tokio_postgres::Statement, Error> {
        match self.statements.entry(query_text.into()) {
            std::collections::hash_map::Entry::Occupied(entry) => Ok(entry.get().clone()),
            std::collections::hash_map::Entry::Vacant(entry) => {
                let statement = self
                    .client
                    .prepare(entry.key())
                    .await
                    .map_err(Error::prepare)?;
                Ok(entry.insert(statement).clone())
            }
        }
    }

    /// Creates a new prepared statement.
    ///
    /// Everything required to prepare the statement is available on the
    /// type argument, so no runtime input is needed:
    ///
    /// ```no_run
    /// # async fn xmain() -> Result<(), aykroyd::tokio_postgres::Error> {
    /// # use aykroyd::{Query, FromRow};
    /// # use aykroyd::tokio_postgres::connect;
    /// # use tokio_postgres::NoTls;
    /// # #[derive(FromRow)]
    /// # pub struct Customer;
    /// #[derive(Query)]
    /// #[aykroyd(row(Customer), text = "
    ///     SELECT id, first, last FROM customers WHERE first = $1
    /// ")]
    /// pub struct GetCustomersByFirstName<'a>(&'a str);
    ///
    /// let (mut client, conn) = connect("host=localhost user=postgres", NoTls).await?;
    ///
    /// // Prepare the query in the database.
    /// client.prepare::<GetCustomersByFirstName>().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn prepare<S: StaticQueryText>(
        &mut self,
    ) -> Result<(), Error> {
        self.prepare_internal(S::QUERY_TEXT).await?;
        Ok(())
    }

    /// Executes a statement, returning the resulting rows.
    ///
    /// We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # async fn xmain() -> Result<(), aykroyd::tokio_postgres::Error> {
    /// # use aykroyd::{Query, FromRow};
    /// # use aykroyd::tokio_postgres::connect;
    /// # use tokio_postgres::NoTls;
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
    /// let (mut client, conn) = connect("host=localhost user=postgres", NoTls).await?;
    ///
    /// // Run the query and iterate over the results.
    /// for customer in client.query(&GetCustomersByFirstName("Sammy")).await? {
    ///     println!("Got customer {} {} with id {}", customer.first, customer.last, customer.id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn query<Q: Query<Self>>(
        &mut self,
        query: &Q,
    ) -> Result<Vec<Q::Row>, Error> {
        let params = query.to_params();
        let params = params.as_ref().map(AsRef::as_ref).unwrap_or(&[][..]);
        let statement = self.prepare_internal(query.query_text()).await?;

        let rows = self
            .client
            .query(&statement, params)
            .await
            .map_err(Error::query)?;

        FromRow::from_rows(&rows)
    }

    /// Executes a statement which returns a single row, returning it.
    ///
    /// Returns an error if the query does not return exactly one row.  We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # async fn xmain() -> Result<(), aykroyd::tokio_postgres::Error> {
    /// # use aykroyd::{QueryOne, FromRow};
    /// # use aykroyd::tokio_postgres::connect;
    /// # use tokio_postgres::NoTls;
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
    /// let (mut client, conn) = connect("host=localhost user=postgres", NoTls).await?;
    ///
    /// // Run the query returning a single row.
    /// let customer = client.query_one(&GetCustomerById(42)).await?;
    /// println!("Got customer {} {} with id {}", customer.first, customer.last, customer.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn query_one<Q: QueryOne<Self>>(
        &mut self,
        query: &Q,
    ) -> Result<Q::Row, Error> {
        let params = query.to_params();
        let params = params.as_ref().map(AsRef::as_ref).unwrap_or(&[][..]);
        let statement = self.prepare_internal(query.query_text()).await?;

        let row = self
            .client
            .query_one(&statement, params)
            .await
            .map_err(Error::query)?;

        FromRow::from_row(&row)
    }

    /// Executes a statement which returns zero or one rows, returning it.
    ///
    /// Returns an error if the query returns more than one row.  We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # async fn xmain() -> Result<(), aykroyd::tokio_postgres::Error> {
    /// # use aykroyd::{QueryOne, FromRow};
    /// # use aykroyd::tokio_postgres::connect;
    /// # use tokio_postgres::NoTls;
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
    /// let (mut client, conn) = connect("host=localhost user=postgres", NoTls).await?;
    ///
    /// // Run the query, possibly returning a single row.
    /// if let Some(customer) = client.query_opt(&GetCustomerById(42)).await? {
    ///     println!("Got customer {} {} with id {}", customer.first, customer.last, customer.id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn query_opt<Q: QueryOne<Self>>(
        &mut self,
        query: &Q,
    ) -> Result<Option<Q::Row>, Error> {
        let params = query.to_params();
        let params = params.as_ref().map(AsRef::as_ref).unwrap_or(&[][..]);
        let statement = self.prepare_internal(query.query_text()).await?;

        let row = self
            .client
            .query_opt(&statement, params)
            .await
            .map_err(Error::query)?;

        row.map(|row| FromRow::from_row(&row)).transpose()
    }

    /// Executes a statement, returning the number of rows modified.
    ///
    /// If the statement does not modify any rows (e.g. SELECT), 0 is returned.  We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # async fn xmain() -> Result<(), aykroyd::tokio_postgres::Error> {
    /// # use aykroyd::{Statement};
    /// # use aykroyd::tokio_postgres::connect;
    /// # use tokio_postgres::NoTls;
    /// #[derive(Statement)]
    /// #[aykroyd(text = "
    ///     UPDATE customers SET first = $2, last = $3 WHERE id = $1
    /// ")]
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
    pub async fn execute<S: Statement<Self>>(
        &mut self,
        statement: &S,
    ) -> Result<u64, Error> {
        let params = statement.to_params();
        let params = params.as_ref().map(AsRef::as_ref).unwrap_or(&[][..]);
        let statement = self.prepare_internal(statement.query_text()).await?;

        let rows_affected = self
            .client
            .execute(&statement, &params)
            .await
            .map_err(Error::query)?;

        Ok(rows_affected)
    }

    /// Begins a new database transaction.
    ///
    /// The transaction will roll back by default - use the `commit` method to commit it.
    pub async fn transaction(&mut self) -> Result<Transaction, Error> {
        Ok(Transaction {
            txn: self
                .client
                .transaction()
                .await
                .map_err(Error::transaction)?,
            statements: &mut self.statements,
        })
    }
}

/// An asynchronous PostgreSQL database transaction.
///
/// Transactions will implicitly roll back by default when dropped. Use the
/// `commit` method to commit the changes made in the transaction.
pub struct Transaction<'a> {
    txn: tokio_postgres::Transaction<'a>,
    statements: &'a mut std::collections::HashMap<String, tokio_postgres::Statement>,
}

impl<'a> Transaction<'a> {
    async fn prepare_internal<S: Into<String>>(
        &mut self,
        query_text: S,
    ) -> Result<tokio_postgres::Statement, Error> {
        match self.statements.entry(query_text.into()) {
            std::collections::hash_map::Entry::Occupied(entry) => Ok(entry.get().clone()),
            std::collections::hash_map::Entry::Vacant(entry) => {
                let statement = self
                    .txn
                    .prepare(entry.key())
                    .await
                    .map_err(Error::prepare)?;
                Ok(entry.insert(statement).clone())
            }
        }
    }

    /// Consumes the transaction, committing all changes made within it.
    pub async fn commit(self) -> Result<(), Error> {
        self.txn.commit().await.map_err(Error::transaction)
    }

    /// Rolls the transaction back, discarding all changes made within it.
    ///
    /// This is equivalent to `Transaction`'s `Drop` implementation, but provides any error encountered to the caller.
    pub async fn rollback(self) -> Result<(), Error> {
        self.txn.rollback().await.map_err(Error::transaction)
    }

    /// Creates a new prepared statement.
    ///
    /// Everything required to prepare the statement is available on the
    /// type argument, so no runtime input is needed:
    ///
    /// ```no_run
    /// # async fn xmain() -> Result<(), aykroyd::tokio_postgres::Error> {
    /// # use aykroyd::{Query, FromRow};
    /// # use aykroyd::tokio_postgres::connect;
    /// # use tokio_postgres::NoTls;
    /// # #[derive(FromRow)]
    /// # pub struct Customer;
    /// #[derive(Query)]
    /// #[aykroyd(row(Customer), text = "
    ///     SELECT id, first, last FROM customers WHERE first = $1
    /// ")]
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
    pub async fn prepare<S: StaticQueryText>(
        &mut self,
    ) -> Result<(), Error> {
        self.prepare_internal(S::QUERY_TEXT).await?;
        Ok(())
    }

    /// Executes a statement, returning the resulting rows.
    ///
    /// We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # async fn xmain() -> Result<(), aykroyd::tokio_postgres::Error> {
    /// # use aykroyd::{Query, FromRow};
    /// # use aykroyd::tokio_postgres::connect;
    /// # use tokio_postgres::NoTls;
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
    pub async fn query<Q: Query<Client>>(
        &mut self,
        query: &Q,
    ) -> Result<Vec<Q::Row>, Error> {
        let params = query.to_params();
        let params = params.as_ref().map(AsRef::as_ref).unwrap_or(&[][..]);
        let statement = self.prepare_internal(query.query_text()).await?;

        let rows = self
            .txn
            .query(&statement, &params)
            .await
            .map_err(Error::query)?;

        FromRow::from_rows(&rows)
    }

    /// Executes a statement which returns a single row, returning it.
    ///
    /// Returns an error if the query does not return exactly one row.  We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # async fn xmain() -> Result<(), aykroyd::tokio_postgres::Error> {
    /// # use aykroyd::{QueryOne, FromRow};
    /// # use aykroyd::tokio_postgres::connect;
    /// # use tokio_postgres::NoTls;
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
    /// let (mut client, conn) = connect("host=localhost user=postgres", NoTls).await?;
    /// let mut txn = client.transaction().await?;
    ///
    /// // Run the query returning a single row.
    /// let customer = txn.query_one(&GetCustomerById(42)).await?;
    /// println!("Got customer {} {} with id {}", customer.first, customer.last, customer.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn query_one<Q: QueryOne<Client>>(
        &mut self,
        query: &Q,
    ) -> Result<Q::Row, Error> {
        let params = query.to_params();
        let params = params.as_ref().map(AsRef::as_ref).unwrap_or(&[][..]);
        let statement = self.prepare_internal(query.query_text()).await?;

        let row = self
            .txn
            .query_one(&statement, params)
            .await
            .map_err(Error::query)?;

        FromRow::from_row(&row)
    }

    /// Executes a statement which returns zero or one rows, returning it.
    ///
    /// Returns an error if the query returns more than one row.  We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # async fn xmain() -> Result<(), aykroyd::tokio_postgres::Error> {
    /// # use aykroyd::{QueryOne, FromRow};
    /// # use aykroyd::tokio_postgres::connect;
    /// # use tokio_postgres::NoTls;
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
    pub async fn query_opt<Q: QueryOne<Client>>(
        &mut self,
        query: &Q,
    ) -> Result<Option<Q::Row>, Error> {
        let params = query.to_params();
        let params = params.as_ref().map(AsRef::as_ref).unwrap_or(&[][..]);
        let statement = self.prepare_internal(query.query_text()).await?;

        let row = self
            .txn
            .query_opt(&statement, params)
            .await
            .map_err(Error::query)?;

        row.map(|row| FromRow::from_row(&row)).transpose()
    }

    /// Executes a statement, returning the number of rows modified.
    ///
    /// If the statement does not modify any rows (e.g. SELECT), 0 is returned.  We'll prepare the statement first if we haven't yet.
    ///
    /// ```no_run
    /// # async fn xmain() -> Result<(), aykroyd::tokio_postgres::Error> {
    /// # use aykroyd::{Statement};
    /// # use aykroyd::tokio_postgres::connect;
    /// # use tokio_postgres::NoTls;
    /// #[derive(Statement)]
    /// #[aykroyd(text = "
    ///     UPDATE customers SET first = $2, last = $3 WHERE id = $1
    /// ")]
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
    pub async fn execute<S: Statement<Client>>(
        &mut self,
        statement: &S,
    ) -> Result<u64, Error> {
        let params = statement.to_params();
        let params = params.as_ref().map(AsRef::as_ref).unwrap_or(&[][..]);
        let statement = self.prepare_internal(statement.query_text()).await?;

        let rows_affected = self
            .txn
            .execute(&statement, &params)
            .await
            .map_err(Error::query)?;

        Ok(rows_affected)
    }
}
