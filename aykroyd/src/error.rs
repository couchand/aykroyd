//! Error handling.
//!
//! Errors can happen in one of a few different phases:
//!
//! * connecting to a database
//! * preparing a query
//! * executing a query
//! * retrieving values from the results
//! * transaction control
//!
//! Use the `kind()` method on [`Error`] to find out
//! which step it was.  If we have an underlying database
//! error it can be retrieved with the `inner()` method.

/// An error that occurred when trying to use the database.
#[derive(Debug, Clone)]
pub struct Error<ClientError> {
    message: String,
    kind: ErrorKind,
    inner: Option<ClientError>,
}

impl<ClientError> Error<ClientError> {
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    pub fn inner(&self) -> Option<&ClientError> {
        self.inner.as_ref()
    }

    pub fn from_column_str<S: Into<String>>(message: S, inner: Option<ClientError>) -> Self {
        let kind = ErrorKind::FromColumn;
        let message = message.into();
        Error {
            message,
            kind,
            inner,
        }
    }

    pub fn prepare_str<S: Into<String>>(message: S, inner: Option<ClientError>) -> Self {
        let kind = ErrorKind::Prepare;
        let message = message.into();
        Error {
            message,
            kind,
            inner,
        }
    }

    pub fn query_str<S: Into<String>>(message: S, inner: Option<ClientError>) -> Self {
        let kind = ErrorKind::Query;
        let message = message.into();
        Error {
            message,
            kind,
            inner,
        }
    }

    pub fn connect_str<S: Into<String>>(message: S, inner: Option<ClientError>) -> Self {
        let kind = ErrorKind::Connect;
        let message = message.into();
        Error {
            message,
            kind,
            inner,
        }
    }

    pub fn transaction_str<S: Into<String>>(message: S, inner: Option<ClientError>) -> Self {
        let kind = ErrorKind::Transaction;
        let message = message.into();
        Error {
            message,
            kind,
            inner,
        }
    }
}

impl<ClientError: std::fmt::Display> Error<ClientError> {
    pub fn from_column(inner: ClientError) -> Self {
        let message = inner.to_string();
        Self::from_column_str(message, Some(inner))
    }

    pub fn prepare(inner: ClientError) -> Self {
        let message = inner.to_string();
        Self::prepare_str(message, Some(inner))
    }

    pub fn query(inner: ClientError) -> Self {
        let message = inner.to_string();
        Self::query_str(message, Some(inner))
    }

    pub fn connect(inner: ClientError) -> Self {
        let message = inner.to_string();
        Self::connect_str(message, Some(inner))
    }

    pub fn transaction(inner: ClientError) -> Self {
        let message = inner.to_string();
        Self::transaction_str(message, Some(inner))
    }
}

impl<ClientError: std::error::Error + 'static> std::error::Error for Error<ClientError> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.inner.as_ref().map(|err| err as &(dyn std::error::Error + 'static))
    }
}

/// What operation prompted the error?
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    /// Database error while preparing a query.
    Prepare,

    /// Database error while executing a query.
    Query,

    /// Bad conversion from a database column.
    FromColumn,

    /// Problems connecting to the database.
    Connect,

    /// Error in transaction control.
    Transaction,
}

impl<ClientError> std::fmt::Display for Error<ClientError> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.message.fmt(f)
    }
}
