#[derive(Debug)]
pub enum Error {
    FromSql(String),
    Query(String),
    Prepare(String),
}

pub trait FromSql<Row, Index>: Sized {
    fn get(row: Row, index: Index) -> Result<Self, Error>;
}

impl<T: mysql::prelude::FromValue> FromSql<&mysql::Row, usize> for T {
    fn get(row: &mysql::Row, index: usize) -> Result<Self, Error> {
        row.get_opt(index)
            .ok_or_else(|| Error::FromSql(format!("unknown column {}", index)))?
            .map_err(|e| Error::FromSql(e.to_string()))
    }
}

impl<T: mysql::prelude::FromValue> FromSql<&mysql::Row, &str> for T {
    fn get(row: &mysql::Row, name: &str) -> Result<Self, Error> {
        row.get_opt(name)
            .ok_or_else(|| Error::FromSql(format!("unknown column {}", name)))?
            .map_err(|e| Error::FromSql(e.to_string()))
    }
}

impl<'a, T: rusqlite::types::FromSql> FromSql<&rusqlite::Row<'a>, usize> for T {
    fn get(row: &rusqlite::Row, index: usize) -> Result<Self, Error> {
        row.get(index).map_err(|e| Error::FromSql(e.to_string()))
    }
}

impl<'a, T: rusqlite::types::FromSql> FromSql<&rusqlite::Row<'a>, &str> for T {
    fn get(row: &rusqlite::Row, name: &str) -> Result<Self, Error> {
        row.get(name).map_err(|e| Error::FromSql(e.to_string()))
    }
}

impl<'a, T: tokio_postgres::types::FromSql<'a>> FromSql<&'a tokio_postgres::Row, usize> for T {
    fn get(row: &'a tokio_postgres::Row, index: usize) -> Result<Self, Error> {
        row.try_get(index).map_err(|e| Error::FromSql(e.to_string()))
    }
}

impl<'a, T: tokio_postgres::types::FromSql<'a>> FromSql<&'a tokio_postgres::Row, &str> for T {
    fn get(row: &'a tokio_postgres::Row, name: &str) -> Result<Self, Error> {
        row.try_get(name).map_err(|e| Error::FromSql(e.to_string()))
    }
}

pub struct ColumnsIndexed<Row> {
    row: Row,
    offset: usize,
}

impl<Row: Copy> ColumnsIndexed<Row> {
    pub fn new(row: Row) -> Self {
        ColumnsIndexed {
            row,
            offset: 0,
        }
    }

    pub fn get<T>(&self, index: usize) -> Result<T, Error>
    where
        T: FromSql<Row, usize>,
    {
        FromSql::get(self.row, self.offset + index)
    }

    pub fn child(&self, offset: usize) -> Self {
        let offset = self.offset + offset;
        ColumnsIndexed {
            row: self.row,
            offset,
        }
    }
}

pub struct ColumnsNamed<Row> {
    row: Row,
    prefix: String,
}

impl<Row: Copy> ColumnsNamed<Row> {
    pub fn new(row: Row) -> Self {
        ColumnsNamed {
            row,
            prefix: String::new(),
        }
    }

    pub fn get<T>(&self, index: &str) -> Result<T, Error>
    where
        T: for<'a> FromSql<Row, &'a str>,
    {
        let mut name = self.prefix.clone();
        name.push_str(index);
        FromSql::get(self.row, name.as_ref())
    }

    pub fn child(&self, prefix: &str) -> Self {
        let prefix = {
            let mut s = self.prefix.clone();
            s.push_str(prefix);
            s
        };
        ColumnsNamed {
            row: self.row,
            prefix,
        }
    }
}

pub trait FromColumnsIndexed<Row>: Sized {
    fn from_columns(columns: ColumnsIndexed<Row>) -> Result<Self, Error>;
}

pub trait FromColumnsNamed<Row>: Sized {
    fn from_columns(columns: ColumnsNamed<Row>) -> Result<Self, Error>;
}

pub trait FromRow<Row>: Sized {
    fn from_row(row: &Row) -> Result<Self, Error>;
}

pub trait SqlText {
    fn sql_text(&self) -> String;
}

pub trait StaticSqlText {
    const SQL_TEXT: &'static str;
}

impl<S: StaticSqlText> SqlText for S {
    fn sql_text(&self) -> String {
        Self::SQL_TEXT.into()
    }
}

pub enum EitherQuery<A, B> {
    Left(A),
    Right(B),
}

impl<A: SqlText, B: SqlText> SqlText for EitherQuery<A, B> {
    fn sql_text(&self) -> String {
        match self {
            EitherQuery::Left(a) => a.sql_text(),
            EitherQuery::Right(b) => b.sql_text(),
        }
    }
}

impl<C, R, A, B> Query<C> for EitherQuery<A, B>
where
    C: Client,
    R: for<'a> FromRow<C::Row<'a>>,
    A: Query<C, Row = R>,
    B: Query<C, Row = R>,
{
    type Row = R;

    fn to_params(&self) -> Vec<C::Param<'_>> {
        match self {
            EitherQuery::Left(a) => a.to_params(),
            EitherQuery::Right(b) => b.to_params(),
        }
    }
}

pub trait Client: Sized {
    type Row<'a>;
    type Param<'a>;
}

pub struct PostgresAsyncClient {
    client: tokio_postgres::Client,
    statements: std::collections::HashMap<String, tokio_postgres::Statement>,
}

impl PostgresAsyncClient {
    async fn prepare_internal<S: Into<String>>(
        &mut self,
        sql_text: S,
    ) -> Result<tokio_postgres::Statement, Error> {
        match self.statements.entry(sql_text.into()) {
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

impl Client for mysql::Conn {
    type Row<'a> = mysql::Row;
    type Param<'a> = mysql::Value;
}

impl Client for rusqlite::Connection {
    type Row<'a> = rusqlite::Row<'a>;
    type Param<'a> = &'a dyn rusqlite::types::ToSql;
}

pub trait Query<C: Client>: SqlText + Sync {
    type Row: for<'a> FromRow<C::Row<'a>>;

    fn to_params(&self) -> Vec<C::Param<'_>>;
}

#[async_trait::async_trait]
pub trait AsyncClient: Client {
    async fn query<Q: Query<Self>>(
        &mut self,
        query: &Q,
    ) -> Result<Vec<Q::Row>, Error>;

    async fn prepare<S: StaticSqlText>(&mut self) -> Result<(), Error>;
}

pub trait SyncClient: Client {
    fn query<Q: Query<Self>>(
        &mut self,
        query: &Q,
    ) -> Result<Vec<Q::Row>, Error>;

    fn prepare<S: StaticSqlText>(&mut self) -> Result<(), Error>;
}

#[async_trait::async_trait]
impl AsyncClient for PostgresAsyncClient {
    async fn query<Q: Query<Self>>(
        &mut self,
        query: &Q,
    ) -> Result<Vec<Q::Row>, Error> {
        let params = query.to_params();
        let statement = self.prepare_internal(query.sql_text()).await?;

        let rows = self.client.query(&statement, &params)
            .await
            .map_err(|e| Error::Query(e.to_string()))?;

        rows.iter().map(FromRow::from_row).collect()
    }

    async fn prepare<S: StaticSqlText>(&mut self) -> Result<(), Error> {
        self.prepare_internal(S::SQL_TEXT).await?;
        Ok(())
    }
}

impl SyncClient for mysql::Conn {
    fn query<Q: Query<Self>>(
        &mut self,
        query: &Q,
    ) -> Result<Vec<Q::Row>, Error> {
        let params = query.to_params();
        let params = match params.len() {
            0 => mysql::Params::Empty,
            _ => mysql::Params::Positional(params),
        };

        let rows = mysql::prelude::Queryable::exec(self, &query.sql_text(), params)
            .map_err(|e| Error::Query(e.to_string()))?;

        rows.iter().map(FromRow::from_row).collect()
    }

    fn prepare<S: StaticSqlText>(&mut self) -> Result<(), Error> {
        use mysql::prelude::Queryable;
        self.prep(S::SQL_TEXT).map_err(|e| Error::Prepare(e.to_string()))?;
        Ok(())
    }
}

impl SyncClient for rusqlite::Connection {
    fn query<Q: Query<Self>>(
        &mut self,
        query: &Q,
    ) -> Result<Vec<Q::Row>, Error> {
        let params = query.to_params();

        let mut statement = rusqlite::Connection::prepare_cached(self, &query.sql_text())
            .map_err(|e| Error::Query(e.to_string()))?;

        let mut rows = statement.query(&params[..])
            .map_err(|e| Error::Query(e.to_string()))?;
        
        let mut result = vec![];
        while let Some(row) = rows.next().map_err(|e| Error::Query(e.to_string()))? {
            result.push(FromRow::from_row(row)?);
        }

        Ok(result)
    }

    fn prepare<S: StaticSqlText>(&mut self) -> Result<(), Error> {
        self.prepare_cached(S::SQL_TEXT).map_err(|e| Error::Prepare(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    struct FakeRow {
        columns: Vec<String>,
        tuple: Vec<String>,
    }

    impl FromSql<&FakeRow, usize> for String {
        fn get(row: &FakeRow, index: usize) -> Result<String, Error> {
            row.tuple.get(index).cloned().ok_or(Error::FromSql("not found".into()))
        }
    }

    impl FromSql<&FakeRow, &str> for String {
        fn get(row: &FakeRow, name: &str) -> Result<String, Error> {
            row.columns
                .iter()
                .position(|d| d.eq_ignore_ascii_case(name))
                .and_then(|i| row.tuple.get(i))
                .cloned()
                .ok_or(Error::FromSql("not found".into()))
        }
    }

    struct User {
        name: String,
    }

    impl<Row: Copy> FromColumnsIndexed<Row> for User
    where
        String: FromSql<Row, usize>,
    {
        fn from_columns(columns: ColumnsIndexed<Row>) -> Result<Self, Error> {
            Ok(User {
                name: columns.get(0)?,
            })
        }
    }

    impl<Row: Copy> FromColumnsNamed<Row> for User
    where
        String: for<'a> FromSql<Row, &'a str>,
    {
        fn from_columns(columns: ColumnsNamed<Row>) -> Result<Self, Error> {
            Ok(User {
                name: columns.get("name")?,
            })
        }
    }

    struct PostIndexed {
        text: String,
        user: User,
    }

    impl<Row: Copy> FromRow<Row> for PostIndexed
    where
        String: FromSql<Row, usize>,
    {
        fn from_row(row: &Row) -> Result<Self, Error> {
            FromColumnsIndexed::from_columns(ColumnsIndexed::new(*row))
        }
    }

    impl<Row: Copy> FromColumnsIndexed<Row> for PostIndexed
    where
        String: FromSql<Row, usize>,
        User: FromColumnsIndexed<Row>,
    {
        fn from_columns(columns: ColumnsIndexed<Row>) -> Result<Self, Error> {
            Ok(PostIndexed {
                text: columns.get(0)?,
                user: FromColumnsIndexed::from_columns(columns.child(1))?,
            })
        }
    }

    #[test]
    fn smoke_indexed() {
        let result = FakeRow {
            columns: vec![
                "text".into(),
                "user_name".into(),
            ],
            tuple: vec![
                "my cool post!".into(),
                "Sam Author".into(),
            ],
        };
        let post = PostIndexed::from_row(&&result).unwrap();
        assert_eq!("Sam Author", post.user.name);
        assert_eq!("my cool post!", post.text);
    }

    struct PostNamed {
        text: String,
        user: User,
    }

    impl<Row: Copy> FromRow<Row> for PostNamed
    where
        String: for<'a> FromSql<Row, &'a str>,
    {
        fn from_row(row: &Row) -> Result<Self, Error> {
            FromColumnsNamed::from_columns(ColumnsNamed::new(*row))
        }
    }

    impl<Row: Copy> FromColumnsNamed<Row> for PostNamed
    where
        String: for<'a> FromSql<Row, &'a str>,
        User: FromColumnsNamed<Row>,
    {
        fn from_columns(columns: ColumnsNamed<Row>) -> Result<Self, Error> {
            Ok(PostNamed {
                text: columns.get("text")?,
                user: FromColumnsNamed::from_columns(columns.child("user_"))?,
            })
        }
    }

    #[test]
    fn smoke_named() {
        let result = FakeRow {
            columns: vec![
                "text".into(),
                "user_name".into(),
            ],
            tuple: vec![
                "my cool post!".into(),
                "Sam Author".into(),
            ],
        };
        let post = PostNamed::from_row(&&result).unwrap();
        assert_eq!("Sam Author", post.user.name);
        assert_eq!("my cool post!", post.text);
    }

    struct GetAllPosts;

    impl StaticSqlText for GetAllPosts {
        const SQL_TEXT: &'static str = "SELECT text, user.name user_name FROM post";
    }

    #[test]
    fn smoke_static_text() {
        let query = GetAllPosts;
        assert_eq!(GetAllPosts::SQL_TEXT, query.sql_text());
    }

    struct GetActivePosts;

    impl StaticSqlText for GetActivePosts {
        const SQL_TEXT: &'static str = "SELECT text, user.name user_name FROM post WHERE active";
    }

    #[test]
    fn smoke_dynamic_text() {
        let query: EitherQuery<GetAllPosts, GetActivePosts> = EitherQuery::Right(GetActivePosts);
        assert_eq!(GetActivePosts::SQL_TEXT, query.sql_text());
    }

    struct GetPostsByUser(String);

    impl StaticSqlText for GetPostsByUser {
        const SQL_TEXT: &'static str = "SELECT text, user.name user_name FROM post \
            WHERE user.name = $1";
    }

    impl<C: Client> Query<C> for GetPostsByUser
    where
        for<'a> PostIndexed: FromRow<C::Row<'a>>,
        for<'a> &'a String: Into<C::Param<'a>>,
    {
        type Row = PostIndexed;

        fn to_params(&self) -> Vec<C::Param<'_>> {
            vec![
                Into::into(&self.0),
            ]
        }
    }

    struct FakeClient(Vec<FakeRow>);

    impl Client for FakeClient {
        type Row<'a> = &'a FakeRow;
        type Param<'a> = String;
    }

    #[test]
    fn smoke_to_params() {
        let query = GetPostsByUser("foobar".into());
        let row = <GetPostsByUser as Query<FakeClient>>::to_params(&query);
        assert_eq!(1, row.len());
        assert_eq!("foobar", row[0]);
    }

    impl SyncClient for FakeClient {
        fn query<Q: Query<Self>>(
            &mut self,
            _query: &Q,
        ) -> Result<Vec<Q::Row>, Error> {
            let mut rows = vec![];
            for row in &self.0 {
                rows.push(FromRow::from_row(&row)?);
            }
            Ok(rows)
        }

        fn prepare<S: StaticSqlText>(&mut self) -> Result<(), Error> {
            Ok(())
        }
    }

    #[test]
    fn smoke_query() {
        let query = GetPostsByUser("Sam Author".into());
        let row = FakeRow {
            columns: vec![
                "text".into(),
                "user_name".into(),
            ],
            tuple: vec![
                "my cool post!".into(),
                "Sam Author".into(),
            ],
        };
        let mut client = FakeClient(vec![row]);

        let result = client.query(&query);

        assert!(matches!(result, Ok(_)));
        if let Ok(rows) = result {
            assert_eq!(1, rows.len());
            assert_eq!("Sam Author", rows[0].user.name);
            assert_eq!("my cool post!", rows[0].text);
        }
    }
}
