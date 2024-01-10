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

    pub fn get<T>(&self, index: usize) -> Result<T, Error<C::Error>>
    where
        T: FromColumnIndexed<C>,
    {
        FromColumnIndexed::from_column(self.row, self.offset + index)
    }

    pub fn get_nested<T>(&self, offset: usize) -> Result<T, Error<C::Error>>
    where
        T: FromColumnsIndexed<C>,
    {
        FromColumnsIndexed::from_columns(self.child(offset))
    }

    fn child(&self, offset: usize) -> Self {
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

    pub fn get<T>(&self, name: &str) -> Result<T, Error<C::Error>>
    where
        T: FromColumnNamed<C>,
    {
        let name = {
            let mut s = self.prefix.clone();
            s.push_str(name);
            s
        };
        FromColumnNamed::from_column(self.row, name.as_ref())
    }

    pub fn get_nested<T>(&self, prefix: &str) -> Result<T, Error<C::Error>>
    where
        T: FromColumnsNamed<C>,
    {
        FromColumnsNamed::from_columns(self.child(prefix))
    }

    fn child(&self, prefix: &str) -> Self {
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
    const NUM_COLUMNS: usize;
    fn from_columns(columns: ColumnsIndexed<C>) -> Result<Self, Error<C::Error>>;
}

impl<C: Client, T: FromColumnsIndexed<C>> FromColumnsIndexed<C> for Option<T> {
    const NUM_COLUMNS: usize = T::NUM_COLUMNS;
    fn from_columns(columns: ColumnsIndexed<C>) -> Result<Self, Error<C::Error>> {
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
    fn from_columns(columns: ColumnsNamed<C>) -> Result<Self, Error<C::Error>>;
}

#[cfg(feature = "derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
pub use aykroyd_v2_derive::{FromColumnsIndexed, FromColumnsNamed};

impl<C, T0, T1> FromColumnsIndexed<C> for (T0, T1)
where
    C: Client,
    T0: FromColumnIndexed<C>,
    T1: FromColumnIndexed<C>,
{
    const NUM_COLUMNS: usize = 2;

    fn from_columns(columns: ColumnsIndexed<C>) -> Result<Self, Error<C::Error>> {
        Ok((columns.get(0)?, columns.get(1)?))
    }
}

impl<C, T0, T1> crate::FromRow<C> for (T0, T1)
where
    C: Client,
    T0: FromColumnIndexed<C>,
    T1: FromColumnIndexed<C>,
{
    fn from_row(row: &C::Row<'_>) -> Result<Self, Error<C::Error>> {
        FromColumnsIndexed::from_columns(ColumnsIndexed::new(row))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::sync_client::{self, TestClient};

    #[test]
    fn columns_indexed_get() {
        fn test<'a, 'b>(columns: &ColumnsIndexed<'a, 'b, TestClient>, index: usize, expected: &str) {
            let actual: String = columns.get(index).unwrap();
            assert_eq!(expected, actual);
        }

        let mut client = TestClient::new();
        let row = client.row(sync_client::RowInner {
            names: vec![
                "name".into(),
                "age".into(),
                "superpower".into(),
            ],
            values: vec![
                "Hermes".into(),
                "42".into(),
                "Filing".into(),
            ],
        });
        let columns = ColumnsIndexed::new(&row);

        test(&columns, 0, "Hermes");
        test(&columns, 1, "42");
        test(&columns, 2, "Filing");
    }

    #[test]
    fn columns_indexed_get_nested() {
        #[derive(PartialEq, Eq, Debug)]
        struct Nested(String, String, String);
        impl FromColumnsIndexed<TestClient> for Nested {
            const NUM_COLUMNS: usize = 3;

            fn from_columns(columns: ColumnsIndexed<TestClient>) -> sync_client::Result<Self> {
                Ok(Nested(
                    columns.get(0)?,
                    columns.get(1)?,
                    columns.get(2)?,
                ))
            }
        }

        fn test<'a, 'b>(columns: &ColumnsIndexed<'a, 'b, TestClient>, expected: Nested) {
            let actual: Nested = columns.get_nested(1).unwrap();
            assert_eq!(expected, actual);
        }

        let mut client = TestClient::new();
        let row = client.row(sync_client::RowInner {
            names: vec![
                "something_else".into(),
                "character_name".into(),
                "character_age".into(),
                "character_superpower".into(),
            ],
            values: vec![
                "Hello".into(),
                "Hermes".into(),
                "42".into(),
                "Filing".into(),
            ],
        });
        let columns = ColumnsIndexed::new(&row);

        test(&columns, Nested("Hermes".into(), "42".into(), "Filing".into()));
    }

    #[test]
    fn columns_named_get() {
        fn test<'a, 'b>(columns: &ColumnsNamed<'a, 'b, TestClient>, name: &str, expected: &str) {
            let actual: String = columns.get(name).unwrap();
            assert_eq!(expected, actual);
        }

        let mut client = TestClient::new();
        let row = client.row(sync_client::RowInner {
            names: vec![
                "name".into(),
                "age".into(),
                "superpower".into(),
            ],
            values: vec![
                "Hermes".into(),
                "42".into(),
                "Filing".into(),
            ],
        });
        let columns = ColumnsNamed::new(&row);

        test(&columns, "name", "Hermes");
        test(&columns, "age", "42");
        test(&columns, "superpower", "Filing");
    }

    #[test]
    fn columns_named_get_nested() {
        #[derive(PartialEq, Eq, Debug)]
        struct Nested(String, String, String);
        impl FromColumnsNamed<TestClient> for Nested {
            fn from_columns(columns: ColumnsNamed<TestClient>) -> sync_client::Result<Self> {
                Ok(Nested(
                    columns.get("name")?,
                    columns.get("age")?,
                    columns.get("superpower")?,
                ))
            }
        }

        fn test<'a, 'b>(columns: &ColumnsNamed<'a, 'b, TestClient>, expected: Nested) {
            let actual: Nested = columns.get_nested("character_").unwrap();
            assert_eq!(expected, actual);
        }

        let mut client = TestClient::new();
        let row = client.row(sync_client::RowInner {
            names: vec![
                "something_else".into(),
                "character_name".into(),
                "character_age".into(),
                "character_superpower".into(),
            ],
            values: vec![
                "Hello".into(),
                "Hermes".into(),
                "42".into(),
                "Filing".into(),
            ],
        });
        let columns = ColumnsNamed::new(&row);

        test(&columns, Nested("Hermes".into(), "42".into(), "Filing".into()));
    }
}
