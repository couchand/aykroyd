//! An opinionated micro-ORM for Rust.
//!
//! Database queries are represented by a plain Rust struct that implements
//! either [`Statement`](query::Statement) or [`Query`](query::Query) (and
//! maybe [`QueryOne`](query::QueryOne)).  The traits `Statement` and `Query`
//! share two common parent traits:
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
//! * [`FromRow`](row::FromRow), to be deserialized from database rows.
//!
//! All of these traits can be derived automatically, such that usual
//! gnarly database access code is reduced to simple struct definitions.
//! These structs logically bind the query text to the input and output types.
//!
//! The binding is not magic, there is no verification against a database.
//! `Query` and `Statement` implementations are an assertion by the developer,
//! one that you would be wise to verify.  It is recommended to write a
//! suite of automated tests which can be run against any database tier.

pub mod client;
pub mod combinator;
pub mod error;
pub mod query;
pub mod row;

#[cfg(feature = "mysql")]
pub mod mysql;
#[cfg(feature = "postgres")]
pub mod postgres;
#[cfg(feature = "tokio-postgres")]
pub mod tokio_postgres;
#[cfg(feature = "rusqlite")]
pub mod rusqlite;

#[cfg(test)]
mod test;

pub use error::Error;
pub use query::{Query, QueryOne, Statement};
pub use row::FromRow;
