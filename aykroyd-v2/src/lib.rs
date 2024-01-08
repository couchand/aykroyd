//! Aykroyd: An opinionated micro-ORM.

pub mod client;
pub mod combinator;
pub mod error;
pub mod query;
pub mod row;

#[cfg(feature = "mysql")]
pub mod mysql;
#[cfg(any(feature = "postgres", feature = "tokio-postgres"))]
pub mod postgres;
#[cfg(feature = "rusqlite")]
pub mod sqlite;

#[cfg(test)]
mod test;

pub use query::{Query, QueryOne, Statement};
pub use row::FromRow;
