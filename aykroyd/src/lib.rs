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
use aykroyd::{FromRow, Query, Statement};

#[derive(Statement)]
#[aykroyd(text = "
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
#[aykroyd(row(Pet), text = "
    SELECT id, name, species FROM pets
")]
struct GetAllPets;
```
"##
)]
//!
//! Once you have a `Statement` or `Query` in hand, you'll need a database
//! connection to run it.  The driver is a [`Client`](client::Client), and
//! it could be synchronous or asynchronous, implementing the methods in
//! the [client specification](client::specification);
//!
//! Aykroyd supports the following database client crates:
//!
//! | DB | Backend Crate | Feature | Sync/Async | Client |
//! | -- | ------------- | ------- | ---------- | ------ |
//! | PostgreSQL | [postgres](https://crates.io/crates/postgres) | `postgres` | Sync | [`aykroyd::postgres::Client`](postgres::Client) |
//! | PostgreSQL | [tokio-postgres](https://crates.io/crates/tokio-postgres) | `tokio-postgres` | Async | [`aykroyd::tokio_postgres::Client`](tokio_postgres::Client) |
//! | MySQL/MariaDB | [mysql](https://crates.io/crates/mysql) | `mysql` | Sync | [`aykroyd::mysql::Client`](mysql::Client) |
//! | SQLite | [rusqlite](https://crates.io/crates/rusqlite) | `rusqlite` | Sync | [`aykroyd::rusqlite::Client`](rusqlite::Client) |
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
use aykroyd::tokio_postgres::{connect, Client};

# use aykroyd::{FromRow, Query, Statement};
#
# #[derive(Statement)]
# #[aykroyd(text = "
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
# #[aykroyd(row(Pet), text = "
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
# impl From<aykroyd::Error<tokio_postgres::Error>> for MyError {
#     fn from(error: aykroyd::Error<tokio_postgres::Error>) -> Self {
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
use aykroyd::postgres::Client;

# use aykroyd::{FromRow, Query, Statement};
#
# #[derive(Statement)]
# #[aykroyd(text = "
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
# #[aykroyd(row(Pet), text = "
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
# impl From<aykroyd::Error<tokio_postgres::Error>> for MyError {
#     fn from(error: aykroyd::Error<tokio_postgres::Error>) -> Self {
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
#[cfg(feature = "rusqlite")]
#[cfg_attr(docsrs, doc(cfg(feature = "rusqlite")))]
pub mod rusqlite;
#[cfg(feature = "tokio-postgres")]
#[cfg_attr(docsrs, doc(cfg(feature = "tokio-postgres")))]
pub mod tokio_postgres;

#[cfg(test)]
mod test;

pub use error::Error;

mod traits;
pub use traits::*;

#[cfg(feature = "derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
pub use aykroyd_derive::{FromRow, Query, QueryOne, Statement};
