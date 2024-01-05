use super::{Client, SqlText, FromRow};

pub trait ToParams<C: Client>: Sync {
    fn to_params(&self) -> Vec<C::Param<'_>>;
}

pub trait Statement<C: Client>: ToParams<C> + SqlText + Sync {}

pub trait Query<C: Client>: ToParams<C> + SqlText + Sync {
    type Row: for<'a> FromRow<C::Row<'a>>;
}

pub trait QueryOne<C: Client>: Query<C> {}
