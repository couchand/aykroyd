//! Query combinators.
//!
//! Say you have some row type.
#![cfg_attr(
    feature = "derive",
    doc = r##"

```
# use aykroyd_v2::FromRow;
#[derive(Debug, FromRow)]
struct Tree {
    height: f32,
    leaves: u64,
    name: String,
}
```
"##)]
//!
//! Then you might have a few different ways you
//! could query for your data.
#![cfg_attr(
    feature = "derive",
    doc = r##"

```
# use aykroyd_v2::{FromRow, Query};
# #[derive(FromRow)] struct Tree;
#[derive(Query)]
#[aykroyd(row(Tree), text = "
    SELECT name, height, leaves FROM trees
    WHERE height > $1
")]
struct GetTreesOver(f32);

#[derive(Query)]
#[aykroyd(row(Tree), text = "
    SELECT name, height, leaves FROM trees
    WHERE name = $1
")]
struct GetTreesNamed<'a>(&'a str);
```
"##)]
//!
//! You might then find yourself in a situation where
//! you want to hold on to a `Tree` query, but you don't
//! care which one you have.  That's what `Either` is for.
//!
//! ```
//! # use aykroyd_v2::client::Client;
//! # use aykroyd_v2::query::{StaticQueryText, ToParams};
//! # use aykroyd_v2::{FromRow, Query};
//! # #[derive(Debug)]
//! # struct Tree;
//! # impl<C: Client> FromRow<C> for Tree {
//! #     fn from_row(_row: &C::Row<'_>) -> Result<Self, aykroyd_v2::Error<C::Error>> {
//! #         Ok(Tree)
//! #     }
//! # }
//! # struct GetTreesOver(f32);
//! # impl StaticQueryText for GetTreesOver {
//! #     const QUERY_TEXT: &'static str = "";
//! # }
//! # impl<C: Client> ToParams<C> for GetTreesOver {
//! #     fn to_params(&self) -> Vec<C::Param<'_>> {
//! #         vec![]
//! #     }
//! # }
//! # impl<C: Client> Query<C> for GetTreesOver {
//! #     type Row = Tree;
//! # }
//! # struct GetTreesNamed<'a>(&'a str);
//! # impl<'a> StaticQueryText for GetTreesNamed<'a> {
//! #     const QUERY_TEXT: &'static str = "";
//! # }
//! # impl<'a, C: Client> ToParams<C> for GetTreesNamed<'a> {
//! #     fn to_params(&self) -> Vec<C::Param<'_>> {
//! #         vec![]
//! #     }
//! # }
//! # impl<'a, C: Client> Query<C> for GetTreesNamed<'a> {
//! #     type Row = Tree;
//! # }
//! # struct DbConn;
//! # impl DbConn {
//! #     fn query(
//! #         &mut self,
//! #         _: &Either<GetTreesOver, GetTreesNamed>,
//! #     ) -> Result<Vec<Tree>, String> {
//! #         Ok(vec![])
//! #     }
//! # }
//! # let mut client = DbConn;
//! use aykroyd_v2::combinator::Either;
//!
//! fn query_and_log(
//!     client: &mut DbConn,
//!     query: Either<GetTreesOver, GetTreesNamed>,
//! ) {
//!     let trees: Vec<Tree> = client.query(&query).unwrap();
//!     println!("Got trees: {trees:?}");
//! }
//!
//! // Run one type of query returning trees over 12.0 units.
//! query_and_log(&mut client, Either::Left(GetTreesOver(12.0)));
//!
//! // Run a different query returning trees named "Bob".
//! query_and_log(&mut client, Either::Right(GetTreesNamed("Bob")));
//! ```

use crate::client::Client;
use crate::query::{QueryText, ToParams};
use crate::{FromRow, Query, QueryOne, Statement};

/// A query that could be one of two options.
///
/// See the [module docs](crate::combinator) for more details.
pub enum Either<A, B> {
    Left(A),
    Right(B),
}

impl<A: QueryText, B: QueryText> QueryText for Either<A, B> {
    fn query_text(&self) -> String {
        match self {
            Either::Left(a) => a.query_text(),
            Either::Right(b) => b.query_text(),
        }
    }
}

impl<C, A, B> ToParams<C> for Either<A, B>
where
    C: Client,
    A: ToParams<C>,
    B: ToParams<C>,
{
    fn to_params(&self) -> Vec<C::Param<'_>> {
        match self {
            Either::Left(a) => a.to_params(),
            Either::Right(b) => b.to_params(),
        }
    }
}

impl<C, A, B> Statement<C> for Either<A, B>
where
    C: Client,
    A: Statement<C>,
    B: Statement<C>,
{
}

impl<C, R, A, B> Query<C> for Either<A, B>
where
    C: Client,
    R: FromRow<C>,
    A: Query<C, Row = R>,
    B: Query<C, Row = R>,
{
    type Row = R;
}

impl<C, R, A, B> QueryOne<C> for Either<A, B>
where
    C: Client,
    R: FromRow<C>,
    A: QueryOne<C, Row = R>,
    B: QueryOne<C, Row = R>,
{
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::client::ToParam;
    use crate::query::StaticQueryText;
    use crate::test::sync_client::{self, TestClient};

    #[test]
    fn query_text() {
        struct A;
        impl StaticQueryText for A {
            const QUERY_TEXT: &'static str = "A";
        }

        struct B;
        impl StaticQueryText for B {
            const QUERY_TEXT: &'static str = "B";
        }

        fn test(either: &Either<A, B>, expected: &str) {
            let query_text = QueryText::query_text(either);
            assert_eq!(expected, query_text);
        }

        test(&Either::Left(A), "A");
        test(&Either::Right(B), "B");
    }

    #[test]
    fn to_params() {
        struct Param(usize);
        impl sync_client::ToParam for Param {
            fn to_param(&self) -> String {
                format!("{}", self.0)
            }
        }

        struct A;
        impl ToParams<TestClient> for A {
            fn to_params(&self) -> Vec<<TestClient as Client>::Param<'_>> {
                vec![ToParam::to_param(&Param(1))]
            }
        }

        struct B;
        impl ToParams<TestClient> for B {
            fn to_params(&self) -> Vec<<TestClient as Client>::Param<'_>> {
                vec![ToParam::to_param(&Param(2))]
            }
        }

        fn test(either: &Either<A, B>, expected: &str) {
            let params = ToParams::to_params(either);
            assert_eq!(1, params.len());
            assert_eq!(expected, params[0].to_param());
        }

        test(&Either::Left(A), "1");
        test(&Either::Right(B), "2");
    }

    #[test]
    fn statement() {
        struct A;
        impl ToParams<TestClient> for A {
            fn to_params(&self) -> Vec<<TestClient as Client>::Param<'_>> {
                vec![]
            }
        }
        impl StaticQueryText for A {
            const QUERY_TEXT: &'static str = "A";
        }
        impl Statement<TestClient> for A {}

        struct B;
        impl ToParams<TestClient> for B {
            fn to_params(&self) -> Vec<<TestClient as Client>::Param<'_>> {
                vec![]
            }
        }
        impl StaticQueryText for B {
            const QUERY_TEXT: &'static str = "B";
        }
        impl Statement<TestClient> for B {}

        fn test<S: Statement<TestClient>>(statement: &S, expected: &str) {
            let mut client = TestClient::new();
            client.execute(statement).unwrap();

            let records = client.records();
            assert_eq!(1, records.len());
            assert_eq!(expected, records[0].text);
        }

        test::<Either<A, B>>(&Either::Left(A), "A");
        test::<Either<A, B>>(&Either::Right(B), "B");
    }

    #[test]
    fn query() {
        struct Row;
        impl FromRow<TestClient> for Row {
            fn from_row(_row: &<TestClient as Client>::Row<'_>) -> sync_client::Result<Self> {
                Ok(Row)
            }
        }

        struct A;
        impl ToParams<TestClient> for A {
            fn to_params(&self) -> Vec<<TestClient as Client>::Param<'_>> {
                vec![]
            }
        }
        impl StaticQueryText for A {
            const QUERY_TEXT: &'static str = "A";
        }
        impl Query<TestClient> for A {
            type Row = Row;
        }

        struct B;
        impl ToParams<TestClient> for B {
            fn to_params(&self) -> Vec<<TestClient as Client>::Param<'_>> {
                vec![]
            }
        }
        impl StaticQueryText for B {
            const QUERY_TEXT: &'static str = "B";
        }
        impl Query<TestClient> for B {
            type Row = Row;
        }

        fn test<Q: Query<TestClient>>(query: &Q, expected: &str) {
            let mut client = TestClient::new();
            client.push_query_result(Ok(vec![]));
            client.query(query).unwrap();

            let records = client.records();
            assert_eq!(1, records.len());
            assert_eq!(expected, records[0].text);
        }

        test::<Either<A, B>>(&Either::Left(A), "A");
        test::<Either<A, B>>(&Either::Right(B), "B");
    }

    #[test]
    fn query_one() {
        struct Row;
        impl FromRow<TestClient> for Row {
            fn from_row(_row: &<TestClient as Client>::Row<'_>) -> sync_client::Result<Self> {
                Ok(Row)
            }
        }

        struct A;
        impl ToParams<TestClient> for A {
            fn to_params(&self) -> Vec<<TestClient as Client>::Param<'_>> {
                vec![]
            }
        }
        impl StaticQueryText for A {
            const QUERY_TEXT: &'static str = "A";
        }
        impl Query<TestClient> for A {
            type Row = Row;
        }
        impl QueryOne<TestClient> for A {}

        struct B;
        impl ToParams<TestClient> for B {
            fn to_params(&self) -> Vec<<TestClient as Client>::Param<'_>> {
                vec![]
            }
        }
        impl StaticQueryText for B {
            const QUERY_TEXT: &'static str = "B";
        }
        impl Query<TestClient> for B {
            type Row = Row;
        }
        impl QueryOne<TestClient> for B {}

        fn test<Q: QueryOne<TestClient>>(query: &Q, expected: &str) {
            let mut client = TestClient::new();
            client.push_query_one_result(Ok(sync_client::RowInner::default()));
            client.query_one(query).unwrap();
            client.push_query_opt_result(Ok(None));
            client.query_opt(query).unwrap();

            let records = client.records();
            assert_eq!(2, records.len());
            assert_eq!(expected, records[0].text);
            assert_eq!(expected, records[1].text);
        }

        test::<Either<A, B>>(&Either::Left(A), "A");
        test::<Either<A, B>>(&Either::Right(B), "B");
    }
}
