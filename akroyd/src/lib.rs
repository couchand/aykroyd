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

#[cfg_attr(feature = "async", async_trait::async_trait)]
pub trait Query: ToRow {
    #[cfg(not(feature = "async"))]
    type Output: FromRow;
    #[cfg(feature = "async")]
    type Output: FromRow + Send;

    const TEXT: &'static str;

    #[cfg(feature = "sync")]
    fn run(&self, client: &mut postgres::Client) -> Result<Vec<Self::Output>, tokio_postgres::Error> {
        use ::postgres::fallible_iterator::FallibleIterator;

        let mut res = vec![];

        let mut it = client.query_raw(Self::TEXT, self.to_row())?;

        while let Some(row) = it.next()? {
            res.push(FromRow::from_row(&row));
        }

        Ok(res)
    }

    #[cfg(feature = "async")]
    async fn run(&self, client: &tokio_postgres::Client) -> Result<Vec<Self::Output>, tokio_postgres::Error> {
        use futures_util::{pin_mut, TryStreamExt};

        let mut res = vec![];

        let it = client.query_raw(Self::TEXT, self.to_row()).await?;

        pin_mut!(it);
        while let Some(row) = it.try_next().await? {
            res.push(FromRow::from_row(&row));
        }

        Ok(res)
    }
}

pub async fn query<Q: Query>(
    client: &tokio_postgres::Client,
    query: &Q,
) -> Result<Vec<Q::Output>, tokio_postgres::Error> {
    use futures_util::{pin_mut, TryStreamExt};

    let mut res = vec![];

    let it = client.query_raw(Q::TEXT, query.to_row()).await?;

    pin_mut!(it);
    while let Some(row) = it.try_next().await? {
        res.push(FromRow::from_row(&row));
    }

    Ok(res)
}

#[cfg(feature = "postgres")]
pub trait PostgresExt {
    fn run<Q: Query>(&mut self, query: &Q) -> Result<Vec<Q::Output>, tokio_postgres::Error>;
}

#[cfg(feature = "postgres")]
impl PostgresExt for &mut postgres::Client {
    fn run<Q: Query>(&mut self, query: &Q) -> Result<Vec<Q::Output>, tokio_postgres::Error> {
        use ::postgres::fallible_iterator::FallibleIterator;

        let mut res = vec![];

        let mut it = self.query_raw(Q::TEXT, query.to_row())?;

        while let Some(row) = it.next()? {
            res.push(FromRow::from_row(&row));
        }

        Ok(res)
    }
}

#[doc(hidden)]
pub mod types {
    pub use tokio_postgres::types::ToSql;
    pub use tokio_postgres::Row;
}
