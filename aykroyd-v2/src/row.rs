//! Traits and structs for handling result rows.

use crate::client::{Client, FromColumnIndexed, FromColumnNamed};
use crate::error::Error;

/// The columns of a result row by index.
pub struct ColumnsIndexed<'a, 'b, C: Client> {
    row: &'a C::Row<'b>,
    offset: usize,
}

impl<'a, 'b, C: Client> ColumnsIndexed<'a, 'b, C> {
    pub fn new(row: &'a C::Row<'b>) -> Self {
        ColumnsIndexed { row, offset: 0 }
    }

    pub fn get<T>(&self, index: usize) -> Result<T, Error>
    where
        T: FromColumnIndexed<C::Row<'b>>,
    {
        FromColumnIndexed::from_column(self.row, self.offset + index)
    }

    pub fn child(&self, offset: usize) -> Self {
        let offset = self.offset + offset;
        ColumnsIndexed {
            row: self.row,
            offset,
        }
    }
}

/// The columns of a result row by name.
pub struct ColumnsNamed<'a, 'b, C: Client> {
    row: &'a C::Row<'b>,
    prefix: String,
}

impl<'a, 'b, C: Client> ColumnsNamed<'a, 'b, C> {
    pub fn new(row: &'a C::Row<'b>) -> Self {
        ColumnsNamed {
            row,
            prefix: String::new(),
        }
    }

    pub fn get<T>(&self, name: &str) -> Result<T, Error>
    where
        T: FromColumnNamed<C::Row<'b>>,
    {
        let name = {
            let mut s = self.prefix.clone();
            s.push_str(name);
            s
        };
        FromColumnNamed::from_column(self.row, name.as_ref())
    }

    pub fn child(&self, prefix: &str) -> Self {
        let prefix = {
            let mut s = self.prefix.clone();
            s.push_str(prefix);
            s
        };
        ColumnsNamed {
            row: self.row,
            prefix,
        }
    }
}

/// A type that can be produced from a result row by column index.
///
/// This is automatically generated by the `FromRow` derive macro
/// when one of the following is true:
///
/// - the type is a tuple struct without attributes
/// - one or more column has an attribute `#[aykroyd(index = <index>)]`
/// - the type has an attribute `#[aykroyd(indexed)]`
pub trait FromColumnsIndexed<C: Client>: Sized {
    //const NUM_COLUMNS: usize;
    fn from_columns(columns: ColumnsIndexed<C>) -> Result<Self, Error>;
}

impl<C: Client, T: FromColumnsIndexed<C>> FromColumnsIndexed<C> for Option<T> {
    //const NUM_COLUMNS: usize = T::NUM_COLUMNS;
    fn from_columns(columns: ColumnsIndexed<C>) -> Result<Self, Error> {
        T::from_columns(columns).map(Some).or(Ok(None)) // TODO: this is terrible!
    }
}

/// A type that can be produced from a result row by column name.
///
/// This is automatically generated by the `FromRow` derive macro
/// when one of the following is true:
///
/// - the type is a struct without attributes
/// - one or more column has an attribute `#[aykroyd(name = "<name>")]`
/// - the type has an attribute `#[aykroyd(named)]`
pub trait FromColumnsNamed<C: Client>: Sized {
    fn from_columns(columns: ColumnsNamed<C>) -> Result<Self, Error>;
}

/// A type that can be produced from a database's result row.
///
/// Don't implement this directly, use the derive macro.
pub trait FromRow<C: Client>: Sized {
    fn from_row(row: &C::Row<'_>) -> Result<Self, Error>;

    fn from_rows(rows: &[C::Row<'_>]) -> Result<Vec<Self>, Error> {
        rows.iter().map(|row| FromRow::from_row(row)).collect()
    }
}
