#[cfg(feature = "derive")]
pub use akroyd_derive::*;

pub trait FromRow {
    fn from_row(row: &tokio_postgres::Row) -> Self;
}

impl FromRow for () {
    fn from_row(_row: &tokio_postgres::Row) -> Self {
        ()
    }
}

pub trait ToRow {
    fn to_row(&self) -> Vec<&(dyn tokio_postgres::types::ToSql + Sync)>;
}

pub trait Query: ToRow {
    type Output: FromRow + Send;

    const TEXT: &'static str;
}

pub trait QueryOne: ToRow {
    type Output: FromRow + Send;

    const TEXT: &'static str;
}

#[cfg(feature = "async")]
#[async_trait::async_trait]
pub trait TokioPostgresExt {
    async fn run<Q: Query + Sync>(&self, query: &Q) -> Result<Vec<Q::Output>, tokio_postgres::Error>;
    async fn run_one<Q: QueryOne + Sync>(&self, query: &Q) -> Result<Q::Output, tokio_postgres::Error>;
}

#[cfg(feature = "async")]
#[async_trait::async_trait]
impl TokioPostgresExt for tokio_postgres::Client {
    async fn run<Q: Query + Sync>(&self, query: &Q) -> Result<Vec<Q::Output>, tokio_postgres::Error> {
        use futures_util::{pin_mut, TryStreamExt};

        let mut res = vec![];

        let it = self.query_raw(Q::TEXT, query.to_row()).await?;

        pin_mut!(it);
        while let Some(row) = it.try_next().await? {
            res.push(FromRow::from_row(&row));
        }

        Ok(res)
    }

    async fn run_one<Q: QueryOne + Sync>(&self, query: &Q) -> Result<Q::Output, tokio_postgres::Error> {
        Ok(FromRow::from_row(&self.query_one(Q::TEXT, &query.to_row()).await?))
    }
}

#[cfg(feature = "postgres")]
pub trait PostgresExt {
    fn run<Q: Query>(&mut self, query: &Q) -> Result<Vec<Q::Output>, tokio_postgres::Error>;
    fn run_one<Q: QueryOne>(&mut self, query: &Q) -> Result<Q::Output, tokio_postgres::Error>;
}

#[cfg(feature = "postgres")]
impl PostgresExt for postgres::Client {
    fn run<Q: Query>(&mut self, query: &Q) -> Result<Vec<Q::Output>, tokio_postgres::Error> {
        use ::postgres::fallible_iterator::FallibleIterator;

        let mut res = vec![];

        let mut it = self.query_raw(Q::TEXT, query.to_row())?;

        while let Some(row) = it.next()? {
            res.push(FromRow::from_row(&row));
        }

        Ok(res)
    }

    fn run_one<Q: QueryOne>(&mut self, query: &Q) -> Result<Q::Output, tokio_postgres::Error> {
        Ok(FromRow::from_row(&self.query_one(Q::TEXT, &query.to_row())?))
    }
}

#[doc(hidden)]
pub mod types {
    pub use tokio_postgres::types::ToSql;
    pub use tokio_postgres::Row;
}
