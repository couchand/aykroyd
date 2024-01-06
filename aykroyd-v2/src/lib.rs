//! Aykroyd: An opinionated micro-ORM.

pub mod client;
pub mod combinator;
pub mod error;
pub mod query;
pub mod row;

#[cfg(feature = "mysql")]
pub mod mysql;
pub mod postgres;
#[cfg(feature = "rusqlite")]
pub mod sqlite;

#[cfg(test)]
mod test;

pub use client::Client;
pub use error::Error;
pub use query::{Query, QueryOne, Statement, StaticQueryText};
pub use row::{FromRow, FromSql};
