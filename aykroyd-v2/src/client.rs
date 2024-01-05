use super::{Error, Query, QueryOne, Statement, StaticQueryText};

pub trait Client: Sized {
    type Row<'a>;
    type Param<'a>;
}

#[async_trait::async_trait]
pub trait AsyncClient: Client {
    async fn query<Q: Query<Self>>(
        &mut self,
        query: &Q,
    ) -> Result<Vec<Q::Row>, Error>;

    async fn execute<S: Statement<Self>>(
        &mut self,
        statement: &S,
    ) -> Result<u64, Error>;

    async fn prepare<S: StaticQueryText>(&mut self) -> Result<(), Error>;

    async fn query_opt<Q: QueryOne<Self>>(
        &mut self,
        query: &Q,
    ) -> Result<Option<Q::Row>, Error> {
        self.query(query).await.map(|rows| rows.into_iter().next())
    }

    async fn query_one<Q: QueryOne<Self>>(
        &mut self,
        query: &Q,
    ) -> Result<Q::Row, Error> {
        self.query_opt(query).await.map(|row| row.unwrap())
    }
}

pub trait SyncClient: Client {
    fn query<Q: Query<Self>>(
        &mut self,
        query: &Q,
    ) -> Result<Vec<Q::Row>, Error>;

    fn execute<S: Statement<Self>>(
        &mut self,
        statement: &S,
    ) -> Result<u64, Error>;

    fn prepare<S: StaticQueryText>(&mut self) -> Result<(), Error>;

    fn query_opt<Q: QueryOne<Self>>(
        &mut self,
        query: &Q,
    ) -> Result<Option<Q::Row>, Error> {
        self.query(query).map(|rows| rows.into_iter().next())
    }

    fn query_one<Q: QueryOne<Self>>(
        &mut self,
        query: &Q,
    ) -> Result<Q::Row, Error> {
        self.query_opt(query).map(|row| row.unwrap())
    }
}
