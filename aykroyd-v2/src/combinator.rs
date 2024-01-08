//! Query combinators.

use super::query::{Query, QueryOne, QueryText, ToParams};
use super::{Client, FromRow};

/// A query that could be one of two options.
pub enum EitherQuery<A, B> {
    Left(A),
    Right(B),
}

impl<A: QueryText, B: QueryText> QueryText for EitherQuery<A, B> {
    fn query_text(&self) -> String {
        match self {
            EitherQuery::Left(a) => a.query_text(),
            EitherQuery::Right(b) => b.query_text(),
        }
    }
}

impl<C, R, A, B> ToParams<C> for EitherQuery<A, B>
where
    C: Client,
    R: FromRow<C>,
    A: Query<C, Row = R>,
    B: Query<C, Row = R>,
{
    fn to_params(&self) -> Vec<C::Param<'_>> {
        match self {
            EitherQuery::Left(a) => a.to_params(),
            EitherQuery::Right(b) => b.to_params(),
        }
    }
}

impl<C, R, A, B> Query<C> for EitherQuery<A, B>
where
    C: Client,
    R: FromRow<C>,
    A: Query<C, Row = R>,
    B: Query<C, Row = R>,
{
    type Row = R;
}

impl<C, R, A, B> QueryOne<C> for EitherQuery<A, B>
where
    C: Client,
    R: FromRow<C>,
    A: QueryOne<C, Row = R>,
    B: QueryOne<C, Row = R>,
{
}
