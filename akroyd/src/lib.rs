#[cfg(feature = "derive")]
pub use akroyd_derive::*;

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

pub trait Execute: Statement {
}

#[cfg(feature = "async")]
#[async_trait::async_trait]
pub trait TokioPostgresExt {
    async fn run<Q: Query + Sync>(&self, query: &Q) -> Result<Vec<Q::Row>, tokio_postgres::Error>;
    async fn run_one<Q: QueryOne + Sync>(&self, query: &Q) -> Result<Q::Row, tokio_postgres::Error>;
    async fn run_opt<Q: QueryOne + Sync>(&self, query: &Q) -> Result<Option<Q::Row>, tokio_postgres::Error>;
    async fn exec<E: Execute + Sync>(&self, statement: &E) -> Result<u64, tokio_postgres::Error>;
}

#[cfg(feature = "async")]
#[async_trait::async_trait]
impl TokioPostgresExt for tokio_postgres::Client {
    async fn run<Q: Query + Sync>(&self, query: &Q) -> Result<Vec<Q::Row>, tokio_postgres::Error> {
        use futures_util::{pin_mut, TryStreamExt};

        let mut res = vec![];

        let it = self.query_raw(Q::TEXT, query.to_row()).await?;

        pin_mut!(it);
        while let Some(row) = it.try_next().await? {
            res.push(FromRow::from_row(row)?);
        }

        Ok(res)
    }

    async fn run_one<Q: QueryOne + Sync>(&self, query: &Q) -> Result<Q::Row, tokio_postgres::Error> {
        Ok(FromRow::from_row(self.query_one(Q::TEXT, &query.to_row()).await?)?)
    }

    async fn run_opt<Q: QueryOne + Sync>(&self, query: &Q) -> Result<Option<Q::Row>, tokio_postgres::Error> {
        Ok(self.query_opt(Q::TEXT, &query.to_row()).await?.map(FromRow::from_row).transpose()?)
    }

    async fn exec<E: Execute + Sync>(&self, statement: &E) -> Result<u64, tokio_postgres::Error> {
        self.execute(E::TEXT, &statement.to_row()).await
    }
}

#[cfg(feature = "async")]
pub struct AsyncClient {
    client: tokio_postgres::Client,
    statements: std::collections::HashMap<StatementKey, tokio_postgres::Statement>,
}

#[cfg(feature = "async")]
pub async fn connect<T>(config: &str, tls: T) -> Result<(AsyncClient, tokio_postgres::Connection<tokio_postgres::Socket, T::Stream>), tokio_postgres::Error>
where T: tokio_postgres::tls::MakeTlsConnect<tokio_postgres::Socket>,
{
    let (client, connection) = tokio_postgres::connect(config, tls).await?;
    let client = AsyncClient::new(client);
    Ok((client, connection))
}

#[cfg(feature = "async")]
type StatementKey = String; // TODO: more

#[cfg(feature = "async")]
impl AsyncClient {
    fn new(client: tokio_postgres::Client) -> Self {
        let statements = std::collections::HashMap::new();
        AsyncClient { client, statements }
    }

    fn statement_key<Q: Statement>() -> StatementKey {
        Q::TEXT.to_string()
    }

    async fn find_or_prepare<Q: Statement>(&mut self) -> Result<tokio_postgres::Statement, tokio_postgres::Error> {
        let key = AsyncClient::statement_key::<Q>();

        if self.statements.get(&key).is_none() {
            println!("preparing {key}!");
            let key = key.clone();
            let prepared = self.client.prepare(Q::TEXT).await?;
            self.statements.insert(key, prepared);
        }

        Ok(self.statements.get(&key).unwrap().clone())
    }

    pub async fn prepare<Q: Statement>(&mut self) -> Result<(), tokio_postgres::Error> {
        self.find_or_prepare::<Q>().await?;
        Ok(())
    }

    pub async fn query<Q: Query>(&mut self, query: &Q) -> Result<Vec<Q::Row>, tokio_postgres::Error> {
        let stmt = self.find_or_prepare::<Q>().await?;
        Ok(self.client.query(&stmt, &query.to_row()).await?.into_iter().map(FromRow::from_row).collect::<Result<Vec<_>, _>>()?)
    }

    pub async fn query_one<Q: QueryOne>(&mut self, query: &Q) -> Result<Q::Row, tokio_postgres::Error> {
        let stmt = self.find_or_prepare::<Q>().await?;
        Ok(FromRow::from_row(self.client.query_one(&stmt, &query.to_row()).await?)?)
    }

    pub async fn query_opt<Q: QueryOne>(&mut self, query: &Q) -> Result<Option<Q::Row>, tokio_postgres::Error> {
        let stmt = self.find_or_prepare::<Q>().await?;
        Ok(self.client.query_opt(&stmt, &query.to_row()).await?.map(FromRow::from_row).transpose()?)
    }

    pub async fn execute<Q: Execute>(&mut self, query: &Q) -> Result<u64, tokio_postgres::Error> {
        let stmt = self.find_or_prepare::<Q>().await?;
        Ok(self.client.execute(&stmt, &query.to_row()).await?)
    }

    pub async fn batch_execute(&self, statements: &str) -> Result<(), tokio_postgres::Error> {
        self.client.batch_execute(statements).await
    }
}

#[cfg(feature = "sync")]
pub trait PostgresExt {
    fn run<Q: Query>(&mut self, query: &Q) -> Result<Vec<Q::Row>, tokio_postgres::Error>;
    fn run_one<Q: QueryOne>(&mut self, query: &Q) -> Result<Q::Row, tokio_postgres::Error>;
    fn run_opt<Q: QueryOne>(&mut self, query: &Q) -> Result<Option<Q::Row>, tokio_postgres::Error>;
    fn exec<E: Execute>(&mut self, statement: &E) -> Result<u64, tokio_postgres::Error>;
}

#[cfg(feature = "sync")]
impl PostgresExt for postgres::Client {
    fn run<Q: Query>(&mut self, query: &Q) -> Result<Vec<Q::Row>, tokio_postgres::Error> {
        use ::postgres::fallible_iterator::FallibleIterator;

        let mut res = vec![];

        let mut it = self.query_raw(Q::TEXT, query.to_row())?;

        while let Some(row) = it.next()? {
            res.push(FromRow::from_row(row)?);
        }

        Ok(res)
    }

    fn run_one<Q: QueryOne>(&mut self, query: &Q) -> Result<Q::Row, tokio_postgres::Error> {
        Ok(FromRow::from_row(self.query_one(Q::TEXT, &query.to_row())?)?)
    }

    fn run_opt<Q: QueryOne>(&mut self, query: &Q) -> Result<Option<Q::Row>, tokio_postgres::Error> {
        Ok(self.query_opt(Q::TEXT, &query.to_row())?.map(FromRow::from_row).transpose()?)
    }

    fn exec<E: Execute>(&mut self, statement: &E) -> Result<u64, tokio_postgres::Error> {
        self.execute(E::TEXT, &statement.to_row())
    }
}

#[doc(hidden)]
pub mod types {
    pub use tokio_postgres::types::ToSql;
    pub use tokio_postgres::{Error, Row};
}
