//! Aykroyd: An opinionated micro-ORM.

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

pub use query::{Query, QueryOne, Statement};
pub use row::FromRow;
