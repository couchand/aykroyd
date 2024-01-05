//! Aykroyd: An opinionated micro-ORM.

pub mod client;
pub mod combinator;
pub mod error;
pub mod query;
pub mod row;

pub mod mysql;
pub mod postgres;
pub mod sqlite;

#[cfg(test)]
mod test;

pub use client::Client;
pub use error::Error;
pub use query::{Query, QueryOne, Statement, StaticQueryText};
pub use row::{FromRow, FromSql};
