//! Traits that represent database clients.
//!
//! Our model expects each database backend to manage the
//! connection as well as caching prepared statements.
//! The client can be synchronous or asynchronous, both
//! provide the same interface.

use crate::error::Error;
use crate::query::{Query, QueryOne, Statement, StaticQueryText};

/// A database client's parameter and row types.
pub trait Client: Sized {
    type Row<'a>;
    type Param<'a>;
}

/// A type that can be retrieved from a database column by index.
pub trait FromColumnIndexed<Row>: Sized {
    fn from_column(row: &Row, index: usize) -> Result<Self, Error>;
}

/// A type that can be retrieved from a database column by name.
pub trait FromColumnNamed<Row>: Sized {
    fn from_column(row: &Row, name: &str) -> Result<Self, Error>;
}

/// A type that can be converted to a database param.
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
