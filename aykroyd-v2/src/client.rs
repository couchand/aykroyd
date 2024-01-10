//! Traits that represent database clients.
//!
//! Our model expects each database backend to manage the
//! connection as well as caching prepared statements.
//! The client can be synchronous or asynchronous, both
//! provide the same interface.
//!
//! To implement a new database driver, start with
//! [`Client`](./trait.Client.html), where you'll define the
//! database's input `Param` output `Row`, and `Error` types.
//! Then add appropriate implementations of
//! [`ToParam`](./trait.ToParam.html) for
//! anything you can convert to your client `Param` type,
//! as well as
//! [`FromColumnIndexed`](./trait.FromColumnIndexed.html) and/or
//! [`FromColumnNamed`](./trait.FromColumnNamed.html) for
//! anything you can retrieve from a database row (by column
//! index and/or name).  Finally, implement methods according
//! to the [specification].

use crate::error::Error;

/// A database client's types.
pub trait Client: Sized {
    /// The database's input parameter type.
    type Param<'a>;

    /// The database's output row type.
    type Row<'a>;

    /// The type of database errors.
    type Error;
}

/// A type that can be retrieved from a database column by index.
pub trait FromColumnIndexed<C: Client>: Sized {
    /// Get the converted value of the column at the given index.
    fn from_column(row: &C::Row<'_>, index: usize) -> Result<Self, Error<C::Error>>;
}

/// A type that can be retrieved from a database column by name.
pub trait FromColumnNamed<C: Client>: Sized {
    /// Get the converted value of the column with the given name.
    fn from_column(row: &C::Row<'_>, name: &str) -> Result<Self, Error<C::Error>>;
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

pub mod specification {
    //! The `aykroyd` client specification.
    //!
    //! Aykroyd clients all implement the following specification.
    //! The consistency makes moving from one backend to another easy,
    //! but flexibility is allowed to make the best use of each driver.
    //!
    //! ## `std` Traits
    //!
    //! Aykroyd clients generally wrap another database driver, so they
    //! implement the following traits:
    //!
    //! * [`From<Driver>`], to create a client by wrapping the driver,
    //! * [`AsRef<Driver>`], for shared access to the driver, and
    //! * [`AsMut<Driver>`], for exclusive access to the driver.
    //!
    //! ## Constructors
    //!
    //! For ease of use, Aykroyd clients provide constructors matching
    //! the ones available on the underlying driver, such as
    //! `open_in_memory()` for the `rusqlite` driver, or the `connect()`
    //! function from `tokio-postgres`.
    //!
    //! ## Query Methods
    //!
    //! Methods on the client are synchronous or asynchronous, as
    //! appropriate, and the overall interface looks the same.
    //! Clients implement the following common query methods.
    //!
    //! ### Sync Client Interface
    //!
    //! ```
    //! use aykroyd_v2::client::Client;
    //! use aykroyd_v2::query::StaticQueryText;
    //! use aykroyd_v2::{Error, Query, QueryOne, Statement};
    //!
    //! /// An example of a synchronous database client.
    //! trait SyncClient: Client {
    //!     fn prepare<S: StaticQueryText>(
    //!         &mut self,
    //!     ) -> Result<(), Error<Self::Error>>;
    //!
    //!     fn execute<S: Statement<Self>>(
    //!         &mut self,
    //!         statement: &S,
    //!     ) -> Result<u64, Error<Self::Error>>;
    //!
    //!     fn query<Q: Query<Self>>(
    //!         &mut self,
    //!         query: &Q,
    //!     ) -> Result<Vec<Q::Row>, Error<Self::Error>>;
    //!
    //!     fn query_one<Q: QueryOne<Self>>(
    //!         &mut self,
    //!         query: &Q,
    //!     ) -> Result<Q::Row, Error<Self::Error>>;
    //!
    //!     fn query_opt<Q: QueryOne<Self>>(
    //!         &mut self,
    //!         query: &Q,
    //!     ) -> Result<Option<Q::Row>, Error<Self::Error>>;
    //! }
    //! ```
    //!
    //! ### Async Client Interface
    //!
    //! ```
    //! use aykroyd_v2::client::Client;
    //! use aykroyd_v2::query::StaticQueryText;
    //! use aykroyd_v2::{Error, Query, QueryOne, Statement};
    //!
    //! /// An example of an asynchronous database client.
    //! # #[async_trait::async_trait]
    //! trait AsyncClient: Client {
    //!     async fn prepare<S: StaticQueryText>(
    //!         &mut self,
    //!     ) -> Result<(), Error<Self::Error>>;
    //!
    //!     async fn execute<S: Statement<Self>>(
    //!         &mut self,
    //!         statement: &S,
    //!     ) -> Result<u64, Error<Self::Error>>;
    //!
    //!     async fn query<Q: Query<Self>>(
    //!         &mut self,
    //!         query: &Q,
    //!     ) -> Result<Vec<Q::Row>, Error<Self::Error>>;
    //!
    //!     async fn query_one<Q: QueryOne<Self>>(
    //!         &mut self,
    //!         query: &Q,
    //!     ) -> Result<Q::Row, Error<Self::Error>>;
    //!
    //!     async fn query_opt<Q: QueryOne<Self>>(
    //!         &mut self,
    //!         query: &Q,
    //!     ) -> Result<Option<Q::Row>, Error<Self::Error>>;
    //! }
    //! ```
    //!
    //! ## Transaction Control
    //!
    //! Databases that offer transaction control should follow the
    //! following transactions interface.
    //!
    //! ### Sync Transaction Interface
    //!
    //! ```
    //! use aykroyd_v2::client::Client;
    //! use aykroyd_v2::query::StaticQueryText;
    //! use aykroyd_v2::{Error, Query, QueryOne, Statement};
    //!
    //! trait SyncClient: Client {
    //!     type Transaction: SyncTransaction<Self>;
    //!
    //!     fn transaction(
    //!         &mut self,
    //!     ) -> Result<Self::Transaction, Error<Self::Error>>;
    //! }
    //!
    //! /// An example of a synchronous database transaction.
    //! trait SyncTransaction<C: Client> {
    //!     fn commit(
    //!         self,
    //!     ) -> Result<(), Error<C::Error>>;
    //!
    //!     fn rollback(
    //!         self,
    //!     ) -> Result<(), Error<C::Error>>;
    //!
    //!     fn prepare<S: StaticQueryText>(
    //!         &mut self,
    //!     ) -> Result<(), Error<C::Error>>;
    //!
    //!     fn execute<S: Statement<C>>(
    //!         &mut self,
    //!         statement: &S,
    //!     ) -> Result<u64, Error<C::Error>>;
    //!
    //!     fn query<Q: Query<C>>(
    //!         &mut self,
    //!         query: &Q,
    //!     ) -> Result<Vec<Q::Row>, Error<C::Error>>;
    //!
    //!     fn query_one<Q: QueryOne<C>>(
    //!         &mut self,
    //!         query: &Q,
    //!     ) -> Result<Q::Row, Error<C::Error>>;
    //!
    //!     fn query_opt<Q: QueryOne<C>>(
    //!         &mut self,
    //!         query: &Q,
    //!     ) -> Result<Option<Q::Row>, Error<C::Error>>;
    //! }
    //! ```
    //!
    //! ### Async Transaction Interface
    //!
    //! ```
    //! use aykroyd_v2::client::Client;
    //! use aykroyd_v2::query::StaticQueryText;
    //! use aykroyd_v2::{Error, Query, QueryOne, Statement};
    //!
    //! # #[async_trait::async_trait]
    //! trait AsyncClient: Client {
    //!     type Transaction: AsyncTransaction<Self>;
    //!
    //!     async fn transaction(
    //!         &mut self,
    //!     ) -> Result<Self::Transaction, Error<Self::Error>>;
    //! }
    //!
    //! /// An example of as asynchronous database transaction.
    //! # #[async_trait::async_trait]
    //! trait AsyncTransaction<C: Client> {
    //!     async fn commit(
    //!         self,
    //!     ) -> Result<(), Error<C::Error>>;
    //!
    //!     async fn rollback(
    //!         self,
    //!     ) -> Result<(), Error<C::Error>>;
    //!
    //!     async fn prepare<S: StaticQueryText>(
    //!         &mut self,
    //!     ) -> Result<(), Error<C::Error>>;
    //!
    //!     async fn execute<S: Statement<C>>(
    //!         &mut self,
    //!         statement: &S,
    //!     ) -> Result<u64, Error<C::Error>>;
    //!
    //!     async fn query<Q: Query<C>>(
    //!         &mut self,
    //!         query: &Q,
    //!     ) -> Result<Vec<Q::Row>, Error<C::Error>>;
    //!
    //!     async fn query_one<Q: QueryOne<C>>(
    //!         &mut self,
    //!         query: &Q,
    //!     ) -> Result<Q::Row, Error<C::Error>>;
    //!
    //!     async fn query_opt<Q: QueryOne<C>>(
    //!         &mut self,
    //!         query: &Q,
    //!     ) -> Result<Option<Q::Row>, Error<C::Error>>;
    //! }
    //! ```
}
