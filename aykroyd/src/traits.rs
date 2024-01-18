use crate::client::Client;
use crate::error::Error;
use crate::query::{QueryText, ToParams};
use crate::row::{ColumnsIndexed, FromColumnsIndexed};

/// A type that can be produced from a database's result row.
///
/// This can be generally derived automatically for structs and tuple structs,
/// by delegating to an implementation of
/// [`FromColumnsIndexed`](crate::row::FromColumnsIndexed) or
/// [`FromColumnsNamed`](crate::row::FromColumnsNamed).
///
/// For structs with named fields, default behavior is to match columns by
/// their name in the result row.
#[cfg_attr(
    feature = "derive",
    doc = r##"

```
# use aykroyd::FromRow;
#[derive(FromRow)]
pub struct Customer {
    id: i32,
    first_name: String,
    last_name: String,
}
```
"##
)]
///
/// You can opt-in to loading fields by column order on a struct with
/// named fields, by using the `by_index` attribute.
#[cfg_attr(
    feature = "derive",
    doc = r##"

```
# use aykroyd::FromRow;
#[derive(FromRow)]
#[aykroyd(by_index)]
pub struct Customer {
    id: i32,
    first_name: String,
    last_name: String,
}
```
"##
)]
///
/// For tuple structs, the fields are taken from the row in order.  The
/// order of the query columns must match the tuple struct fields.
#[cfg_attr(
    feature = "derive",
    doc = r##"

```
# use aykroyd::FromRow;
#[derive(FromRow)]
pub struct QueryResults(i32, f32, String);
```
"##
)]
///
/// If you just need the results of an ad-hoc query, consider using an
/// anonymous tuple instead.
#[cfg_attr(
    feature = "derive",
    doc = r##"

```
# use aykroyd::Query;
use rust_decimal::Decimal;

#[derive(Query)]
#[aykroyd(row((i32, Decimal)), text = "
    SELECT EXTRACT(MONTH FROM closed_on), SUM(amount) FROM sales
")]
pub struct SalesByMonth;
```
"##
)]
///
/// If the default mapping is not sufficient, you can control what column
/// the field is taken from.  This is useful for renaming columns:
#[cfg_attr(
    feature = "derive",
    doc = r##"

```
# use aykroyd::FromRow;
#[derive(FromRow)]
pub struct Widget {
    #[aykroyd(column = "type")]
    pub ty: String,
}
```
"##
)]
///
/// You can assign explicit column indexes to each field, too.
#[cfg_attr(
    feature = "derive",
    doc = r##"

```
# use aykroyd::FromRow;
#[derive(FromRow)]
pub struct Widget {
    #[aykroyd(column = 4)]
    pub ty: String,
}
```
"##
)]
///
/// You can also load nested rows, as long as they use the same
/// column loading strategy.  Use this to share models between queries,
/// load associations, etc.
#[cfg_attr(
    feature = "derive",
    doc = r##"

```
# use aykroyd::{FromRow, Query};
# struct Color;
#[derive(FromRow)]
#[aykroyd(by_index)]
struct Person {
    name: String,
    favorite_color: Color,
}

#[derive(FromRow)]
#[aykroyd(by_index)]
struct Pet {
    name: String,
    #[aykroyd(nested)]
    owner: Person,
    #[aykroyd(nested)]
    vet: Option<Person>,
}

#[derive(Query)]
#[aykroyd(row(Pet), text = "
    SELECT pet.name, owner.name, owner.fav_color FROM pets
")]
struct GetPets;
```
"##
)]
///
/// See [`FromColumnsIndexed`] and [`FromColumnsNamed`](crate::row::FromColumnsNamed)
/// for more details.
pub trait FromRow<C: Client>: Sized {
    fn from_row(row: &C::Row<'_>) -> Result<Self, Error<C::Error>>;

    fn from_rows(rows: &[C::Row<'_>]) -> Result<Vec<Self>, Error<C::Error>> {
        rows.iter().map(|row| FromRow::from_row(row)).collect()
    }
}

macro_rules! impl_tuple_from_row {
    (
        $(
            $name:ident
        ),*
        $(,)?
    ) => {
        impl<
            C,
            $(
                $name,
            )*
        > FromRow<C> for ($($name,)*)
        where
            C: Client,
            ($($name,)*): FromColumnsIndexed<C>,
        {
            fn from_row(row: &C::Row<'_>) -> Result<Self, Error<C::Error>> {
                FromColumnsIndexed::from_columns(ColumnsIndexed::new(row))
            }
        }
    };
}

impl_tuple_from_row!();
impl_tuple_from_row!(T0);
impl_tuple_from_row!(T0, T1);
impl_tuple_from_row!(T0, T1, T2);
impl_tuple_from_row!(T0, T1, T2, T3);
impl_tuple_from_row!(T0, T1, T2, T3, T4);
impl_tuple_from_row!(T0, T1, T2, T3, T4, T5);
impl_tuple_from_row!(T0, T1, T2, T3, T4, T5, T6);
impl_tuple_from_row!(T0, T1, T2, T3, T4, T5, T6, T7);

/// A database statement which returns no results.
///
/// A `Statement` is something that has query text and can be
/// converted to the parameters of some database `Client`.
///
/// This can generally be derived automatically for structs.  The source
/// order of the fields corresponds to parameter order: the first field
/// in source order is `$1`, the second `$2`, and so on.
#[cfg_attr(
    feature = "derive",
    doc = r##"

```
# use aykroyd::Statement;
#[derive(Statement)]
#[aykroyd(text = "
    INSERT INTO customers (first_name, last_name) VALUES ($1, $2)
")]
pub struct InsertCustomer<'a> {
    first_name: &'a str,
    last_name: &'a str,
}
```
"##
)]
///
/// For queries with more than a handful of parameters, this can get
/// error-prone. Help ensure that the struct fields and the query text
/// stay in sync by annotating parameter index on the fields.
#[cfg_attr(
    feature = "derive",
    doc = r##"

```
# use aykroyd::Statement;
#[derive(Statement)]
#[aykroyd(text = "
    INSERT INTO customers (first, last, middle)
    VALUES ($1, $2, $3)
")]
pub struct InsertCustomer<'a> {
    #[aykroyd(param = "$1")]
    pub first: &'a str,
    #[aykroyd(param = "$3")]
    pub middle: &'a str,
    #[aykroyd(param = "$2")]
    pub last: &'a str,
}
```
"##
)]
///
/// The query text can be provided inline, as above, or loaded from
/// a file.  The path is relative to a `queries/` directory at the
/// root of the crate.
#[cfg_attr(
    feature = "derive",
    doc = r##"

```
# use aykroyd::Statement;
#[derive(Statement)]
#[aykroyd(file = "insert-customer.sql")]
pub struct InsertCustomer<'a> {
    #[aykroyd(param = "$1")]
    pub first: &'a str,
    #[aykroyd(param = "$3")]
    pub middle: &'a str,
    #[aykroyd(param = "$2")]
    pub last: &'a str,
}
```
"##
)]
pub trait Statement<C: Client>: QueryText + ToParams<C> + Sync {}

/// A database query that returns zero or more result rows.
///
/// A `Query` is something that has `QueryText`, can be converted
/// to the parameters of some database `Client`, and has a result
/// type that can be produced from that `Client`'s rows.
///
/// You can use the derive macro to produce each of these parts:
#[cfg_attr(
    feature = "derive",
    doc = r##"

```
# use aykroyd::{FromRow, Query};
#[derive(FromRow)]
struct Todo {
    id: i32,
    label: String,
}

#[derive(Query)]
#[aykroyd(row(Todo), text = "SELECT id, label FROM todo")]
struct GetAllTodos;
```
"##
)]
///
/// Just as with a [`Statement`], a `Query` can have parameters,
/// taken in source order.
/// For queries with more than a handful of parameters, this can get
/// error-prone. Help ensure that the struct fields and the query text
/// stay in sync by annotating parameter index on the fields.
#[cfg_attr(
    feature = "derive",
    doc = r##"

```
# use aykroyd::Query;
# #[derive(aykroyd::FromRow)]
# struct Pet;
#[derive(Query)]
#[aykroyd(row(Pet), text = "
    SELECT first_name, last_name, species
    FROM pet
    WHERE first_name = $1
    AND last_name = $2
    AND species = $3
")]
struct SearchPets<'a> {
    #[aykroyd(param = "$3")]
    pub species: &'a str,
    #[aykroyd(param = "$1")]
    pub first: &'a str,
    #[aykroyd(param = "$2")]
    pub last: &'a str,
}
```
"##
)]
///
/// The query text can be provided inline, as above, or loaded from
/// a file.  The path is relative to a `queries/` directory at the
/// root of the crate.
#[cfg_attr(
    feature = "derive",
    doc = r##"

```
# use aykroyd::Statement;
#[derive(Statement)]
#[aykroyd(file = "summarize-quarter.sql")]
struct SummarizeQuarter;
```
"##
)]
pub trait Query<C: Client>: QueryText + ToParams<C> + Sync {
    type Row: FromRow<C>;
}

/// A marker trait for a query that returns at most one row.
///
/// A `QueryOne` is a marker trait, indicating that a `Query`
/// will only ever return zero or one row.
///
/// You can use the derive macro to generate an implementation.
#[cfg_attr(
    feature = "derive",
    doc = r##"

```
# use aykroyd::{FromRow, QueryOne};
#[derive(FromRow)]
struct Todo {
    id: i32,
    label: String,
}

#[derive(QueryOne)]
#[aykroyd(row(Todo), text = "
    SELECT id, label FROM todo WHERE id = $1
")]
struct GetTodoById(i32);
```
"##
)]
pub trait QueryOne<C: Client>: Query<C> {}
