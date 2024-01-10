//! Traits to define database queries, and their derive macros.
//!
//! This module contains a group of traits that together provide
//! the tools needed to define database queries.

use crate::client::Client;

/// The text of a given `Query` or `Statement`.
///
/// Most types will get the blanket implementation of
/// this trait for implementors of [`StaticQueryText`].
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
///
/// Query text is trimmed by the derive macro:
#[cfg_attr(
    feature = "derive",
    doc = r##"

```
# use aykroyd::Statement;
# use aykroyd::query::StaticQueryText;
#[derive(Statement)]
#[aykroyd(text = "     A      ")]
struct A;

assert_eq!("A", A::QUERY_TEXT);
```
"##)]
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
    fn to_params(&self) -> Option<Vec<C::Param<'_>>>;
}
