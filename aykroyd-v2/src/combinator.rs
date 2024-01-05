use super::{SqlText, Query, QueryOne, ToParams, FromRow, Client};

pub enum EitherQuery<A, B> {
    Left(A),
    Right(B),
}

impl<A: SqlText, B: SqlText> SqlText for EitherQuery<A, B> {
    fn sql_text(&self) -> String {
        match self {
            EitherQuery::Left(a) => a.sql_text(),
            EitherQuery::Right(b) => b.sql_text(),
        }
    }
}

impl<C, R, A, B> ToParams<C> for EitherQuery<A, B>
where
    C: Client,
    R: for<'a> FromRow<C::Row<'a>>,
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
    R: for<'a> FromRow<C::Row<'a>>,
    A: Query<C, Row = R>,
    B: Query<C, Row = R>,
{
    type Row = R;
}

impl<C, R, A, B> QueryOne<C> for EitherQuery<A, B>
where
    C: Client,
    R: for<'a> FromRow<C::Row<'a>>,
    A: QueryOne<C, Row = R>,
    B: QueryOne<C, Row = R>,
{}
