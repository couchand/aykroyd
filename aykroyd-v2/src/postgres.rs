//! PostgreSQL bindings.

use crate::client::{AsyncClient, Client, FromColumnIndexed, FromColumnNamed, ToParam};
use crate::error::Error;
use crate::query::{Query, Statement, StaticQueryText};
use crate::row::FromRow;

impl<T> FromColumnIndexed<PostgresAsyncClient> for T
where
    T: tokio_postgres::types::FromSqlOwned,
{
    fn from_column(row: &tokio_postgres::Row, index: usize) -> Result<Self, Error<tokio_postgres::Error>> {
        row.try_get(index)
            .map_err(Error::from_column)
    }
}

impl<T> FromColumnNamed<PostgresAsyncClient> for T
where
    T: tokio_postgres::types::FromSqlOwned,
{
    fn from_column(row: &tokio_postgres::Row, name: &str) -> Result<Self, Error<tokio_postgres::Error>> {
        row.try_get(name)
            .map_err(Error::from_column)
    }
}

impl<T> ToParam<PostgresAsyncClient> for T
where
    T: tokio_postgres::types::ToSql + Sync,
{
    fn to_param(&self) -> &(dyn tokio_postgres::types::ToSql + Sync) {
        self
    }
}

pub struct PostgresAsyncClient {
    client: tokio_postgres::Client,
    statements: std::collections::HashMap<String, tokio_postgres::Statement>,
}

impl PostgresAsyncClient {
    pub fn new(client: tokio_postgres::Client) -> Self {
        let statements = std::collections::HashMap::new();
        PostgresAsyncClient { client, statements }
    }

    async fn prepare_internal<S: Into<String>>(
        &mut self,
        query_text: S,
    ) -> Result<tokio_postgres::Statement, Error<tokio_postgres::Error>> {
        match self.statements.entry(query_text.into()) {
            std::collections::hash_map::Entry::Occupied(entry) => Ok(entry.get().clone()),
            std::collections::hash_map::Entry::Vacant(entry) => {
                let statement = self
                    .client
                    .prepare(entry.key())
                    .await
                    .map_err(Error::prepare)?;
                Ok(entry.insert(statement).clone())
            }
        }
    }
}

impl AsRef<tokio_postgres::Client> for PostgresAsyncClient {
    fn as_ref(&self) -> &tokio_postgres::Client {
        &self.client
    }
}

impl Client for PostgresAsyncClient {
    type Row<'a> = tokio_postgres::Row;
    type Param<'a> = &'a (dyn tokio_postgres::types::ToSql + Sync);
    type Error = tokio_postgres::Error;
}

#[async_trait::async_trait]
impl AsyncClient for PostgresAsyncClient {
    async fn query<Q: Query<Self>>(&mut self, query: &Q) -> Result<Vec<Q::Row>, Error<tokio_postgres::Error>> {
        let params = query.to_params();
        let statement = self.prepare_internal(query.query_text()).await?;

        let rows = self
            .client
            .query(&statement, &params)
            .await
            .map_err(Error::query)?;

        FromRow::from_rows(&rows)
    }

    async fn execute<S: Statement<Self>>(&mut self, statement: &S) -> Result<u64, Error<tokio_postgres::Error>> {
        let params = statement.to_params();
        let statement = self.prepare_internal(statement.query_text()).await?;

        let rows_affected = self
            .client
            .execute(&statement, &params)
            .await
            .map_err(Error::query)?;

        Ok(rows_affected)
    }

    async fn prepare<S: StaticQueryText>(&mut self) -> Result<(), Error<tokio_postgres::Error>> {
        self.prepare_internal(S::QUERY_TEXT).await?;
        Ok(())
    }
}
