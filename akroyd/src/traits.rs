pub trait FromRow {
    fn from_row(row: tokio_postgres::Row) -> Result<Self, tokio_postgres::Error> where Self: Sized;
}

impl FromRow for () {
    fn from_row(_row: tokio_postgres::Row) -> Result<Self, tokio_postgres::Error> {
        Ok(())
    }
}

impl<A: for<'a> tokio_postgres::types::FromSql<'a>, B: for<'a> tokio_postgres::types::FromSql<'a>> FromRow for (A, B) {
    fn from_row(row: tokio_postgres::Row) -> Result<Self, tokio_postgres::Error> {
        Ok((
            row.try_get(0)?,
            row.try_get(1)?,
        ))
    }
}

pub trait Statement {
    const TEXT: &'static str;

    fn to_row(&self) -> Vec<&(dyn tokio_postgres::types::ToSql + Sync)>;
}

pub trait Query: Statement {
    type Row: FromRow + Send;
}

pub trait QueryOne: Statement {
    type Row: FromRow + Send;
}
