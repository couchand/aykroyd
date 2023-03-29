use crate::*;

#[cfg(feature = "sync")]
pub struct Client {
    client: postgres::Client,
    statements: std::collections::HashMap<StatementKey, tokio_postgres::Statement>,
}

#[cfg(feature = "sync")]
impl Client {
    pub fn connect<T>(params: &str, tls_mode: T) -> Result<Self, tokio_postgres::Error>
    where
        T: postgres::tls::MakeTlsConnect<postgres::Socket> + 'static + Send,
        T::TlsConnect: Send,
        T::Stream: Send,
        <T::TlsConnect as postgres::tls::TlsConnect<postgres::Socket>>::Future: Send,
    {
        let client = postgres::Client::connect(params, tls_mode)?;
        let statements = std::collections::HashMap::new();
        Ok(Client { client, statements })
    }

    fn statement_key<Q: Statement>() -> StatementKey {
        Q::TEXT.to_string()
    }

    fn find_or_prepare<Q: Statement>(&mut self) -> Result<tokio_postgres::Statement, tokio_postgres::Error> {
        let key = Client::statement_key::<Q>();

        if self.statements.get(&key).is_none() {
            let key = key.clone();
            let prepared = self.client.prepare(Q::TEXT)?;
            self.statements.insert(key, prepared);
        }

        Ok(self.statements.get(&key).unwrap().clone())
    }

    pub fn prepare<Q: Statement>(&mut self) -> Result<(), tokio_postgres::Error> {
        self.find_or_prepare::<Q>()?;
        Ok(())
    }

    pub fn query<Q: Query>(&mut self, query: &Q) -> Result<Vec<Q::Row>, tokio_postgres::Error> {
        let stmt = self.find_or_prepare::<Q>()?;
        Ok(self.client.query(&stmt, &query.to_row())?.into_iter().map(FromRow::from_row).collect::<Result<Vec<_>, _>>()?)
    }

    pub fn query_one<Q: QueryOne>(&mut self, query: &Q) -> Result<Q::Row, tokio_postgres::Error> {
        let stmt = self.find_or_prepare::<Q>()?;
        Ok(FromRow::from_row(self.client.query_one(&stmt, &query.to_row())?)?)
    }

    pub fn query_opt<Q: QueryOne>(&mut self, query: &Q) -> Result<Option<Q::Row>, tokio_postgres::Error> {
        let stmt = self.find_or_prepare::<Q>()?;
        Ok(self.client.query_opt(&stmt, &query.to_row())?.map(FromRow::from_row).transpose()?)
    }

    pub fn execute<Q: Statement>(&mut self, query: &Q) -> Result<u64, tokio_postgres::Error> {
        let stmt = self.find_or_prepare::<Q>()?;
        Ok(self.client.execute(&stmt, &query.to_row())?)
    }

    pub fn batch_execute(&mut self, statements: &str) -> Result<(), tokio_postgres::Error> {
        self.client.batch_execute(statements)
    }
}
