/// A trait for types that can be constructed from a `tokio_postgres::Row`.
///
/// This can be generally derived automatically (for structs).
///
/// For structs with named fields, the fields are expected to have the same
/// name as the column in the `Row`.
///
/// ```rust
/// # use akroyd::FromRow;
/// #[derive(FromRow)]
/// pub struct Customer {
///     id: i32,
///     first_name: String,
///     last_name: String,
/// }
/// ```
///
/// For tuple structs, the fields are loaded from the row in order, so the
/// query's column ordering must match the tuple struct's field order.  If you
/// just need the results of an ad-hoc query, consider using an anonymous tuple
/// instead.
///
/// ```rust
/// # use akroyd::FromRow;
/// #[derive(FromRow)]
/// pub struct QueryResults(i32, f32, String);
/// ```
pub trait FromRow: Sized {
    fn from_row(row: tokio_postgres::Row) -> Result<Self, tokio_postgres::Error>;
}

impl FromRow for () {
    fn from_row(_row: tokio_postgres::Row) -> Result<Self, tokio_postgres::Error> {
        Ok(())
    }
}

impl<
        A: for<'a> tokio_postgres::types::FromSql<'a>,
        B: for<'a> tokio_postgres::types::FromSql<'a>,
    > FromRow for (A, B)
{
    fn from_row(row: tokio_postgres::Row) -> Result<Self, tokio_postgres::Error> {
        Ok((row.try_get(0)?, row.try_get(1)?))
    }
}

/// A trait for types that represent a statement or query.
///
/// This can generally be derived automatically (for structs).  If you're deriving
/// `Query` or `QueryOne`, an impl of this trait will also be generated.
///
/// The source order of the fields corresponds to the assignment to parameters.
/// The first field in source order is `$1`, the second `$2`, and so on.
///
/// ```rust
/// # use akroyd::Statement;
/// #[derive(Statement)]
/// #[query(text = "INSERT INTO customers (first_name, last_name) VALUES ($1, $2)")]
/// pub struct InsertCustomer {
///     first_name: String,
///     last_name: String,
/// }
/// ```
pub trait Statement {
    const TEXT: &'static str;

    fn to_row(&self) -> Vec<&(dyn tokio_postgres::types::ToSql + Sync)>;
}

/// A trait for types that represent a query for multiple rows.
///
/// This can generally be derived automatically (for structs).
///
/// The source order of the fields corresponds to the assignment to parameters.
/// The first field in source order is `$1`, the second `$2`, and so on.
/// ```rust
/// # use akroyd::{Query, FromRow};
/// #[derive(Query)]
/// #[query(text = "SELECT id, first, last FROM customers WHERE first = $1", row(Customer))]
/// pub struct GetCustomersByFirstName<'a>(&'a str);
///
/// #[derive(FromRow)]
/// pub struct Customer {
///     id: i32,
///     first: String,
///     last: String,
/// }
/// ```
pub trait Query: Statement {
    type Row: FromRow + Send;
}

/// A trait for types that represent a query that returns at most one row.
///
/// This can generally be derived automatically (for structs).
///
/// The source order of the fields corresponds to the assignment to parameters.
/// The first field in source order is `$1`, the second `$2`, and so on.
/// ```rust
/// # use akroyd::{QueryOne, FromRow};
/// #[derive(QueryOne)]
/// #[query(text = "SELECT id, first, last FROM customers WHERE id = $1", row(Customer))]
/// pub struct GetCustomersById(i32);
///
/// #[derive(FromRow)]
/// pub struct Customer {
///     id: i32,
///     first: String,
///     last: String,
/// }
/// ```
pub trait QueryOne: Statement {
    type Row: FromRow + Send;
}
