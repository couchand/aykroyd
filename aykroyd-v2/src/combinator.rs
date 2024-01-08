//! Query combinators.

use super::query::{Query, QueryOne, QueryText, Statement, ToParams};
use super::{Client, FromRow};

/// A query that could be one of two options.
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
