//! Error handling.

/// An error that occurred when trying to use the database.
#[derive(Debug)]
pub enum Error {
    FromSql(String),
    Query(String),
    Prepare(String),
}
