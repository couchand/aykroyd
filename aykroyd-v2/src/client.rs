//! Traits that represent database clients.
//!
//! Our model expects each database backend to manage the
//! connection as well as caching prepared statements.
//! The client can be synchronous or asynchronous, both
//! provide the same interface.
//!
//! To implement a new database driver, start with
//! [`Client`](./trait.Client.html), where you'll define the
//! database's input `Param` and output `Row` types.
//! Then add appropriate implementations of
//! [`ToParam`](./trait.ToParam.html) for
//! anything you can convert to your client `Param` type,
//! as well as
//! [`FromColumnIndexed`](./trait.FromColumnIndexed.html) and/or
//! [`FromColumnNamed`](./trait.FromColumnNamed.html) for
//! anything you can retrieve from a database row (by column
//! index and/or name).  Finally, implement either
//! [`AsyncClient`](./trait.AsyncClient.html) or
//! [`SyncClient`](./trait.SyncClient.html) as appropriate.

use crate::error::Error;
use crate::query::{Query, QueryOne, Statement, StaticQueryText};

/// A database client's parameter and row types.
pub trait Client: Sized {
    /// The database's input parameter type.
    type Param<'a>;

    /// The database's output row type.
    type Row<'a>;
}

/// A type that can be retrieved from a database column by index.
pub trait FromColumnIndexed<C: Client>: Sized {
    /// Get the converted value of the column at the given index.
    fn from_column<'a>(row: &C::Row<'a>, index: usize) -> Result<Self, Error>;
}

/// A type that can be retrieved from a database column by name.
pub trait FromColumnNamed<C: Client>: Sized {
    /// Get the converted value of the column with the given name.
    fn from_column<'a>(row: &C::Row<'a>, name: &str) -> Result<Self, Error>;
}

/// A type that can be converted to a database param.
///
/// Your database client probably either has an owned object
/// parameter type or a trait that any parameter type can
/// implement. For an example where the parameter is an
/// owned object, see the MySQL implementation.
/// For an example where the parameter is a trait object,
/// see the PostgreSQL implementation.
pub trait ToParam<C: Client> {
    fn to_param(&self) -> C::Param<'_>;
}

/// An asynchronous database client.
#[async_trait::async_trait]
pub trait AsyncClient: Client {
    async fn query<Q: Query<Self>>(&mut self, query: &Q) -> Result<Vec<Q::Row>, Error>;

    async fn execute<S: Statement<Self>>(&mut self, statement: &S) -> Result<u64, Error>;

    async fn prepare<S: StaticQueryText>(&mut self) -> Result<(), Error>;

    async fn query_opt<Q: QueryOne<Self>>(&mut self, query: &Q) -> Result<Option<Q::Row>, Error> {
        self.query(query).await.map(|rows| rows.into_iter().next())
    }

    async fn query_one<Q: QueryOne<Self>>(&mut self, query: &Q) -> Result<Q::Row, Error> {
        self.query_opt(query).await.map(|row| row.unwrap())
    }
}

/// A synchronous database client.
pub trait SyncClient: Client {
    fn query<Q: Query<Self>>(&mut self, query: &Q) -> Result<Vec<Q::Row>, Error>;

    fn execute<S: Statement<Self>>(&mut self, statement: &S) -> Result<u64, Error>;

    fn prepare<S: StaticQueryText>(&mut self) -> Result<(), Error>;

    fn query_opt<Q: QueryOne<Self>>(&mut self, query: &Q) -> Result<Option<Q::Row>, Error> {
        self.query(query).map(|rows| rows.into_iter().next())
    }

    fn query_one<Q: QueryOne<Self>>(&mut self, query: &Q) -> Result<Q::Row, Error> {
        self.query_opt(query).map(|row| row.unwrap())
    }
}
