use super::{Client, FromRow};

pub trait QueryText {
    fn query_text(&self) -> String;
}

pub trait StaticQueryText {
    const QUERY_TEXT: &'static str;
}

impl<S: StaticQueryText> QueryText for S {
    fn query_text(&self) -> String {
        Self::QUERY_TEXT.into()
    }
}

pub trait ToParams<C: Client>: Sync {
    fn to_params(&self) -> Vec<C::Param<'_>>;
}

pub trait Statement<C: Client>: QueryText + ToParams<C> + Sync {}

pub trait Query<C: Client>: QueryText + ToParams<C> + Sync {
    type Row: for<'a> FromRow<C::Row<'a>>;
}

pub trait QueryOne<C: Client>: Query<C> {}
