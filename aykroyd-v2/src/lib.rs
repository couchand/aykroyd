#![cfg_attr(docsrs, feature(doc_cfg))]

//! An opinionated micro-ORM for Rust.
//!
//! Database queries are represented by a plain Rust struct that implements
//! either [`Statement`](Statement) or [`Query`](Query) (and maybe
//! [`QueryOne`](QueryOne)).  The traits `Statement` and `Query` share two
//! common parent traits:
//!
//! * [`QueryText`](query::QueryText), which gives access to the text of the
//!   query, and
//! * [`ToParams`](query::ToParams), which we can use to turn the struct into
//!   database parameters.
//!
//! Using these together, a database client can prepare the text of a query
//! and then run it on a database, passing in the required parameters.
//!
//! In addition, the `Query` trait has an associated type `Row` which must
//! implement:
//!
//! * [`FromRow`], to be deserialized from database rows.
//!
//! All of these traits can be derived automatically, so the usual
//! gnarly database access code is reduced to simple struct definitions.
//! These structs logically bind the query text to input parameters and
//! output row types.
//!
//! The binding is not magic, there is no verification against a database.
//! `Query` and `Statement` implementations are an assertion by the developer,
//! one that you would be wise to verify.  It is recommended to write a
//! suite of automated tests which can be run against any database tier.
#![cfg_attr(
    feature = "derive",
    doc = r##"

```
use aykroyd_v2::{FromRow, Query, Statement};

#[derive(Statement)]
#[aykroyd(query = "
    INSERT INTO pets (name, species) VALUES ($1, $2)
")]
struct InsertPet<'a> {
    name: &'a str,
    species: &'a str,
}

#[derive(FromRow)]
struct Pet {
    id: i32,
    name: String,
    species: String,
}

#[derive(Query)]
#[aykroyd(row(Pet), query = "
    SELECT id, name, species FROM pets
")]
struct GetAllPets;
```
"##
)]
//!
//! Once you have a `Statement` or `Query` in hand, you'll need a database
//! connection to run it.  The driver is a [`Client`](client::Client), and
//! it could be synchronous or asynchronous,
//! implementing [`SyncClient`](client::SyncClient) or
//! [`AsyncClient`](client::AsyncClient).
//!
//! Aykroyd supports the following database client crates:
//!
//! | DB | Backend Crate | Feature | Sync/Async | Client |
//! | -- | ------------- | ------- | ---------- | ------ |
//! | PostgreSQL | [postgres](https://crates.io/crates/postgres) | `postgres` | Sync | [`aykroyd_v2::postgres::Client`](postgres::Client) |
//! | PostgreSQL | [tokio-postgres](https://crates.io/crates/tokio-postgres) | `tokio-postgres` | Async | [`aykroyd_v2::tokio_postgres::Client`](tokio_postgres::Client) |
//! | MySQL/MariaDB | [mysql](https://crates.io/crates/mysql) | `mysql` | Sync | [`aykroyd_v2::mysql::Client`](mysql::Client) |
//! | SQLite | [rusqlite](https://crates.io/crates/rusqlite) | `rusqlite` | Sync | [`aykroyd_v2::rusqlite::Client`](rusqlite::Client) |
//!
//! ## Examples
//!
//! Here's how it might look end-to-end with various clients.
#![cfg_attr(
    feature = "tokio-postgres",
    doc = r##"

The asynchronous PostgreSQL client, available when compiled
with crate feature `tokio-postgres`.

```
use tokio_postgres::NoTls;
use aykroyd_v2::client::AsyncClient;
use aykroyd_v2::tokio_postgres::{connect, Client};

# use aykroyd_v2::{FromRow, Query, Statement};
#
# #[derive(Statement)]
# #[aykroyd(query = "
#     INSERT INTO pets (name, species) VALUES ($1, $2)
# ")]
# struct InsertPet<'a> {
#     name: &'a str,
#     species: &'a str,
# }
#
# #[derive(FromRow)]
# struct Pet {
#     id: i32,
#     name: String,
#     species: String,
# }
#
# #[derive(Query)]
# #[aykroyd(row(Pet), query = "
#     SELECT id, name, species FROM pet
# ")]
# struct GetAllPets;
#
# struct MyError;
# impl From<tokio_postgres::Error> for MyError {
#     fn from(error: tokio_postgres::Error) -> Self {
#         MyError
#     }
# }
# impl From<aykroyd_v2::Error<tokio_postgres::Error>> for MyError {
#     fn from(error: aykroyd_v2::Error<tokio_postgres::Error>) -> Self {
#         MyError
#     }
# }
#
# async fn try_main() -> Result<(), MyError> {
// Connect to the database
let (mut client, conn) =
    connect("host=localhost user=postgres", NoTls).await?;

// As with tokio_postgres, you need to spawn a task for the connection.
tokio::spawn(async move {
    if let Err(e) = conn.await {
        eprintln!("connection error: {e}");
    }
});

// Execute a statement, returning the number of rows modified.
let insert_count = client.execute(&InsertPet {
    name: "Dan",
    species: "Felis catus",
}).await?;
assert_eq!(insert_count, 1);

// Run a query and map the result objects.
let rows = client.query(&GetAllPets).await?;
assert_eq!(rows.len(), 1);
assert_eq!(rows[0].name, "Dan");
#
# Ok(())
# }
```
"##
)]
#![cfg_attr(
    feature = "postgres",
    doc = r##"

The synchronous PostgreSQL client, available when compiled
with crate feature `postgres`.

```no_run
use postgres::NoTls;
use aykroyd_v2::client::SyncClient;
use aykroyd_v2::postgres::Client;

# use aykroyd_v2::{FromRow, Query, Statement};
#
# #[derive(Statement)]
# #[aykroyd(query = "
#     INSERT INTO pets (name, species) VALUES ($1, $2)
# ")]
# struct InsertPet<'a> {
#     name: &'a str,
#     species: &'a str,
# }
#
# #[derive(FromRow)]
# struct Pet {
#     id: i32,
#     name: String,
#     species: String,
# }
#
# #[derive(Query)]
# #[aykroyd(row(Pet), query = "
#     SELECT id, name, species FROM pet
# ")]
# struct GetAllPets;
#
# struct MyError;
# impl From<tokio_postgres::Error> for MyError {
#     fn from(error: tokio_postgres::Error) -> Self {
#         MyError
#     }
# }
# impl From<aykroyd_v2::Error<tokio_postgres::Error>> for MyError {
#     fn from(error: aykroyd_v2::Error<tokio_postgres::Error>) -> Self {
#         MyError
#     }
# }
# fn try_main() -> Result<(), MyError> {
// Connect to the database
let mut client =
    Client::connect("host=localhost user=postgres", NoTls)?;

// Execute a statement, returning the number of rows modified.
let insert_count = client.execute(&InsertPet {
    name: "Dan",
    species: "Felis catus",
})?;
assert_eq!(insert_count, 1);

// Run a query and map the result objects.
let rows = client.query(&GetAllPets)?;
assert_eq!(rows.len(), 1);
assert_eq!(rows[0].name, "Dan");
#
# Ok(())
# }
```
"##
)]

pub mod client;
pub mod combinator;
pub mod error;
pub mod query;
pub mod row;

#[cfg(feature = "mysql")]
#[cfg_attr(docsrs, doc(cfg(feature = "mysql")))]
pub mod mysql;
#[cfg(feature = "postgres")]
#[cfg_attr(docsrs, doc(cfg(feature = "postgres")))]
pub mod postgres;
#[cfg(feature = "tokio-postgres")]
#[cfg_attr(docsrs, doc(cfg(feature = "tokio-postgres")))]
pub mod tokio_postgres;
#[cfg(feature = "rusqlite")]
#[cfg_attr(docsrs, doc(cfg(feature = "rusqlite")))]
pub mod rusqlite;

#[cfg(test)]
mod test;

pub use error::Error;

use crate::client::Client;
use crate::query::{QueryText, ToParams};

#[cfg(feature = "derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
pub use aykroyd_v2_derive::FromRow;

/// A type that can be produced from a database's result row.
///
/// Don't implement this directly, use the derive macro.
pub trait FromRow<C: Client>: Sized {
    fn from_row(row: &C::Row<'_>) -> Result<Self, Error<C::Error>>;

    fn from_rows(rows: &[C::Row<'_>]) -> Result<Vec<Self>, Error<C::Error>> {
        rows.iter().map(|row| FromRow::from_row(row)).collect()
    }
}

/// A database statement which returns no results.
///
/// A `Statement` is something that has `QueryText`, and can be
/// converted to the parameters of some database `Client`.
///
/// You can use the derive macro to produce each of these parts:
///
/// ```ignore
/// #[derive(Statement)]
/// #[aykroyd(text = "UPDATE todo SET label = $1 WHERE id = $2")]
/// struct UpdateTodo(String, isize);
/// ```
pub trait Statement<C: Client>: QueryText + ToParams<C> + Sync {}

/// A database query that returns zero or more result rows.
///
/// A `Query` is something that has `QueryText`, can be converted
/// to the parameters of some database `Client`, and has a result
/// type that can be produced from that `Client`'s rows.
///
/// You can use the derive macro to produce each of these parts:
///
/// ```ignore
/// #[derive(FromRow)]
/// struct Todo {
///     id: isize,
///     label: String,
/// }
///
/// #[derive(Query)]
/// #[aykroyd(row(Todo), text = "SELECT id, label FROM todo")]
/// struct GetAllTodos;
/// ```
pub trait Query<C: Client>: QueryText + ToParams<C> + Sync {
    type Row: FromRow<C>;
}

/// A marker trait that a query only returns zero or one row.
///
/// A `QueryOne` is a marker trait, indicating that a `Query`
/// will only ever return zero or one row.
pub trait QueryOne<C: Client>: Query<C> {}

#[cfg(feature = "derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
pub use aykroyd_v2_derive::{Query, Statement};
