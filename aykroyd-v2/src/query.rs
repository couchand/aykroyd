//! Traits to define database queries, and their derive macros.
//!
//! This module contains a group of traits that together provide
//! the tools needed to define database queries.

use crate::client::Client;
use crate::row::FromRow;

/// The text of a given `Query` or `Statement`.
///
/// Most types will get the blanket implementation of
/// this trait for implementors of `StaticQueryText`.
/// The dynamic version exists, however, to enable
/// query combinators.
pub trait QueryText {
    fn query_text(&self) -> String;
}

/// The constant text of a `Query` or `Statement`.
///
/// Types that implement this trait can be prepared
/// statically, without reference to any particular
/// query parameters.
///
/// Don't implement this trait directly, use the
/// derive macro for `Query` or `Statement`.
pub trait StaticQueryText {
    const QUERY_TEXT: &'static str;
}

impl<S: StaticQueryText> QueryText for S {
    fn query_text(&self) -> String {
        Self::QUERY_TEXT.into()
    }
}

/// A helper trait to build query parameters for a `Client`.
///
/// Types that wish to be used as a `Query` or `Statement`
/// need to be able to be converted to the right
/// parameter type for a given `Client`.
///
/// Don't implement this trait directly, use the
/// derive macro for `Query` or `Statement`.
pub trait ToParams<C: Client>: Sync {
    fn to_params(&self) -> Vec<C::Param<'_>>;
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

#[cfg(feature = "derive")]
pub use aykroyd_v2_derive::Query;
