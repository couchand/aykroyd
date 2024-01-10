use crate::client::Client;
use crate::error::Error;
use crate::query::{QueryText, ToParams};

/// A type that can be produced from a database's result row.
///
/// This can be generally derived automatically for structs and tuple structs,
/// by delegating to an implementation of
/// [`FromColumnsIndexed`](crate::row::FromColumnsIndexed) or
/// [`FromColumnsNamed`](crate::row::FromColumnsNamed).
///
/// For structs with named fields, default behavior is to match columns by
/// their name in the result row.
///
/// ```rust
/// # use aykroyd_v2::FromRow;
/// #[derive(FromRow)]
/// pub struct Customer {
///     id: i32,
///     first_name: String,
///     last_name: String,
/// }
/// ```
///
/// For tuple structs, the fields are taken from the row in order.  The
/// order of the query columns must match the tuple struct fields.
///
/// ```rust
/// # use aykroyd_v2::FromRow;
/// #[derive(FromRow)]
/// pub struct QueryResults(i32, f32, String);
/// ```
///
/// You can opt-in to loading fields by column order on a struct with
/// named fields, by using the `by_index` attribute.
///
/// ```rust
/// # use aykroyd_v2::FromRow;
/// #[derive(FromRow)]
/// #[aykroyd(by_index)]
/// pub struct Customer {
///     id: i32,
///     first_name: String,
///     last_name: String,
/// }
/// ```
///
/// If you just need the results of an ad-hoc query, consider using an
/// anonymous tuple instead.
///
/// ```rust
/// # use aykroyd_v2::Query;
/// use rust_decimal::Decimal;
///
/// #[derive(Query)]
/// #[aykroyd(
///     query = "SELECT EXTRACT(MONTH FROM closed_on), SUM(amount) FROM sales",
///     row((i32, Decimal))
/// )]
/// pub struct SalesByMonth;
/// ```
pub trait FromRow<C: Client>: Sized {
    fn from_row(row: &C::Row<'_>) -> Result<Self, Error<C::Error>>;

    fn from_rows(rows: &[C::Row<'_>]) -> Result<Vec<Self>, Error<C::Error>> {
        rows.iter().map(|row| FromRow::from_row(row)).collect()
    }
}

/// A database statement which returns no results.
///
/// A `Statement` is something that has `QueryText`, and can be
/// converted to the parameters of some database `Client`.
///
/// You can use the derive macro to produce each of these parts:
///
/// ```ignore
/// #[derive(Statement)]
/// #[aykroyd(text = "UPDATE todo SET label = $1 WHERE id = $2")]
/// struct UpdateTodo(String, isize);
/// ```
pub trait Statement<C: Client>: QueryText + ToParams<C> + Sync {}

/// A database query that returns zero or more result rows.
///
/// A `Query` is something that has `QueryText`, can be converted
/// to the parameters of some database `Client`, and has a result
/// type that can be produced from that `Client`'s rows.
///
/// You can use the derive macro to produce each of these parts:
///
/// ```ignore
/// #[derive(FromRow)]
/// struct Todo {
///     id: isize,
///     label: String,
/// }
///
/// #[derive(Query)]
/// #[aykroyd(row(Todo), text = "SELECT id, label FROM todo")]
/// struct GetAllTodos;
/// ```
pub trait Query<C: Client>: QueryText + ToParams<C> + Sync {
    type Row: FromRow<C>;
}

/// A marker trait that a query only returns zero or one row.
///
/// A `QueryOne` is a marker trait, indicating that a `Query`
/// will only ever return zero or one row.
pub trait QueryOne<C: Client>: Query<C> {}
