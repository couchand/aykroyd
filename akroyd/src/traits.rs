/// A trait for types that can be constructed from a `tokio_postgres::Row`.
///
/// This can be generally derived automatically (for structs).
///
/// For structs with named fields, the names must match exactly the name of
/// the column in a result `Row`.
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
/// For tuple structs, the fields are taken from the row in order.  The
/// order of the query columns must match the tuple struct fields.
///
/// ```rust
/// # use akroyd::FromRow;
/// #[derive(FromRow)]
/// pub struct QueryResults(i32, f32, String);
/// ```
///
/// If you just need the results of an ad-hoc query, consider using an
/// anonymous tuple instead.
///
/// ```rust
/// # use akroyd::Query;
/// # use rust_decimal::Decimal;
/// #[derive(Query)]
/// #[query(
///     text = "SELECT EXTRACT(MONTH FROM closed_on), SUM(amount) FROM sales",
///     row((i32, Decimal))
/// )]
/// pub struct SalesByMonth;
/// ```
///
/// If the default mapping is not sufficient, you can control what column
/// the field is taken from.  This is most useful for renaming columns:
///
/// ```rust
/// # use akroyd::FromRow;
/// #[derive(FromRow)]
/// pub struct Widget {
///     #[query(column = "type")]
///     pub ty: String,
/// }
/// ```
///
/// You can also (somewhat questionably) assign an explicit column index.
/// Before doing so, consider whether this is the best approach to solving
/// your problem, as it will likely be confusing to use.
///
/// ```rust
/// # use akroyd::FromRow;
/// #[derive(FromRow)]
/// pub struct Widget {
///     #[query(column = 4)]
///     pub ty: String,
/// }
/// ```
pub trait FromRow: Sized {
    /// Build the type from a PostgreSQL result row.
    fn from_row(row: tokio_postgres::Row) -> Result<Self, tokio_postgres::Error>;
}

impl FromRow for () {
    fn from_row(_row: tokio_postgres::Row) -> Result<Self, tokio_postgres::Error> {
        Ok(())
    }
}

macro_rules! impl_tuple_from_row {
    (
        $(
            $name:ident $index:literal$(,)?
        )+
    ) => {
        impl<
            $(
                $name: for<'a> tokio_postgres::types::FromSql<'a>,
            )+
        > FromRow for ($($name,)+) {
            fn from_row(row: tokio_postgres::Row) -> Result<Self, tokio_postgres::Error> {
                Ok((
                    $(
                        row.try_get($index)?,
                    )+
                ))
            }
        }
    };
}

impl_tuple_from_row!(A 0);
impl_tuple_from_row!(A 0, B 1);
impl_tuple_from_row!(A 0, B 1, C 2);
impl_tuple_from_row!(A 0, B 1, C 2, D 3);
impl_tuple_from_row!(A 0, B 1, C 2, D 3, E 4);
impl_tuple_from_row!(A 0, B 1, C 2, D 3, E 4, F 5);
impl_tuple_from_row!(A 0, B 1, C 2, D 3, E 4, F 5, G 6);
impl_tuple_from_row!(A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7);

/// A SQL statement or query, with typed parameters.
///
/// This can generally be derived automatically (for structs).  If you're deriving
/// `Query` or `QueryOne`, don't derive this, an implementation of this trait will
/// be generated for you.
///
/// The source order of the fields corresponds to parameter order: the first field
/// in source order is `$1`, the second `$2`, and so on.
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
///
/// For queries with more than a handful of parameters, this can get error-prone.
/// Help ensure that the struct fields and the query text stay in sync by annotating
/// parameter index on the fields:
///
/// ```rust
/// # use akroyd::Statement;
/// #[derive(Statement)]
/// #[query(text = "
///     INSERT INTO customers (first, last, middle, salutation)
///     VALUES ($1, $2, $3, $4)
/// ")]
/// pub struct InsertCustomer<'a> {
///     #[query(param = "$4")]
///     pub salutation: &'a str,
///     #[query(param = "$1")]
///     pub first: &'a str,
///     #[query(param = "$3")]
///     pub middle: &'a str,
///     #[query(param = "$2")]
///     pub last: &'a str,
/// }
/// ```
pub trait Statement {
    /// The SQL text of the statement or query.
    const TEXT: &'static str;

    /// Type of the statement's result rows.
    type Row: FromRow + Send;

    /// Prepare the instance's parameters for serialization.
    fn to_row(&self) -> Vec<&(dyn tokio_postgres::types::ToSql + Sync)>;
}

/// A marker trait for a query that may return any number of rows.
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
///
/// For queries with more than a handful of parameters, this can get error-prone.
/// Help ensure that the struct fields and the query text stay in sync by annotating
/// parameter index on the fields:
///
/// ```rust
/// # use akroyd::{Query, FromRow};
/// # #[derive(FromRow)]
/// # pub struct Customer {
/// #     id: i32,
/// #     first: String,
/// #     last: String,
/// # }
/// #[derive(Query)]
/// #[query(row(Customer), text = "
///     SELECT id, first, last
///     FROM customers
///     WHERE first = $1 OR last = $2 OR middle = $3 OR salutation = $4
/// ")]
/// pub struct FuzzySearch<'a> {
///     #[query(param = "$4")]
///     pub salutation: &'a str,
///     #[query(param = "$1")]
///     pub first: &'a str,
///     #[query(param = "$3")]
///     pub middle: &'a str,
///     #[query(param = "$2")]
///     pub last: &'a str,
/// }
/// ```
pub trait Query: Statement {}

/// A marker trait for a query that returns at most one row.
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
///
/// For queries with more than a handful of parameters, this can get error-prone.
/// Help ensure that the struct fields and the query text stay in sync by annotating
/// parameter index on the fields:
///
/// ```rust
/// # use akroyd::{QueryOne, FromRow};
/// # #[derive(FromRow)]
/// # pub struct Customer {
/// #     id: i32,
/// #     first: String,
/// #     last: String,
/// # }
/// #[derive(QueryOne)]
/// #[query(row(Customer), text = "
///     SELECT id, first, last
///     FROM customers
///     WHERE first = $1 AND last = $2 AND middle = $3 AND salutation = $4
/// ")]
/// pub struct ExactSearch<'a> {
///     #[query(param = "$4")]
///     pub salutation: &'a str,
///     #[query(param = "$1")]
///     pub first: &'a str,
///     #[query(param = "$3")]
///     pub middle: &'a str,
///     #[query(param = "$2")]
///     pub last: &'a str,
/// }
/// ```
pub trait QueryOne: Statement {}
