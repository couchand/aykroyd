//! Error handling.

/// An error that occurred when trying to use the database.
#[derive(Debug)]
pub enum Error {
    /// Bad conversion from a database column.
    FromColumn(String),

    /// Database error while preparing a query.
    Prepare(String),

    /// Database error while executing a query.
    Query(String),
}
