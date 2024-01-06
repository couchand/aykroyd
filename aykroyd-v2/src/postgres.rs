//! PostgreSQL bindings.

use super::client::AsyncClient;
use super::{Client, Error, FromRow, FromSql, Query, Statement, StaticQueryText};

impl<'a, T: tokio_postgres::types::FromSql<'a>> FromSql<&'a tokio_postgres::Row, usize> for T {
    fn get(row: &'a tokio_postgres::Row, index: usize) -> Result<Self, Error> {
        row.try_get(index)
            .map_err(|e| Error::FromSql(e.to_string()))
    }
}

impl<'a, T: tokio_postgres::types::FromSql<'a>> FromSql<&'a tokio_postgres::Row, &str> for T {
    fn get(row: &'a tokio_postgres::Row, name: &str) -> Result<Self, Error> {
        row.try_get(name).map_err(|e| Error::FromSql(e.to_string()))
    }
}

pub struct PostgresAsyncClient {
    client: tokio_postgres::Client,
    statements: std::collections::HashMap<String, tokio_postgres::Statement>,
}

impl PostgresAsyncClient {
    async fn prepare_internal<S: Into<String>>(
        &mut self,
        query_text: S,
    ) -> Result<tokio_postgres::Statement, Error> {
        match self.statements.entry(query_text.into()) {
            std::collections::hash_map::Entry::Occupied(entry) => Ok(entry.get().clone()),
            std::collections::hash_map::Entry::Vacant(entry) => {
                let statement = self
                    .client
                    .prepare(entry.key())
                    .await
                    .map_err(|e| Error::Prepare(e.to_string()))?;
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
}

#[async_trait::async_trait]
impl AsyncClient for PostgresAsyncClient {
    async fn query<Q: Query<Self>>(&mut self, query: &Q) -> Result<Vec<Q::Row>, Error> {
        let params = query.to_params();
        let statement = self.prepare_internal(query.query_text()).await?;

        let rows = self
            .client
            .query(&statement, &params)
            .await
            .map_err(|e| Error::Query(e.to_string()))?;

        rows.iter().map(FromRow::from_row).collect()
    }

    async fn execute<S: Statement<Self>>(&mut self, statement: &S) -> Result<u64, Error> {
        let params = statement.to_params();
        let statement = self.prepare_internal(statement.query_text()).await?;

        let rows_affected = self
            .client
            .execute(&statement, &params)
            .await
            .map_err(|e| Error::Query(e.to_string()))?;

        Ok(rows_affected)
    }

    async fn prepare<S: StaticQueryText>(&mut self) -> Result<(), Error> {
        self.prepare_internal(S::QUERY_TEXT).await?;
        Ok(())
    }
}
