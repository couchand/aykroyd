#[derive(Debug)]
pub enum Error {
    FromSql(String),
    Database(String),
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

pub trait Client: Sized {
    type Row<'a>;
    type Param<'a>;
}

impl Client for tokio_postgres::Client {
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

pub trait Query<C: Client>: SqlText {
    type Row: for<'a> FromRow<C::Row<'a>>;

    fn to_row(&self) -> Vec<C::Param<'_>>;
}

#[async_trait::async_trait]
pub trait AsyncClient: Client {
    async fn query<Q: Query<Self> + Sync>(
        &mut self,
        query: &Q,
    ) -> Result<Vec<Q::Row>, Error>;
}

pub trait SyncClient: Client {
    fn query<Q: Query<Self> + Sync>(
        &mut self,
        query: &Q,
    ) -> Result<Vec<Q::Row>, Error>;
}

#[async_trait::async_trait]
impl AsyncClient for tokio_postgres::Client {
    async fn query<Q: Query<Self> + Sync>(
        &mut self,
        query: &Q,
    ) -> Result<Vec<Q::Row>, Error> {
        let rows = tokio_postgres::Client::query(self, &query.sql_text(), &[])
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        rows.iter().map(FromRow::from_row).collect()
    }
}

impl SyncClient for mysql::Conn {
    fn query<Q: Query<Self> + Sync>(
        &mut self,
        query: &Q,
    ) -> Result<Vec<Q::Row>, Error> {
        let rows = mysql::prelude::Queryable::query(self, &query.sql_text())
            .map_err(|e| Error::Database(e.to_string()))?;

        rows.iter().map(FromRow::from_row).collect()
    }
}

impl SyncClient for rusqlite::Connection {
    fn query<Q: Query<Self> + Sync>(
        &mut self,
        query: &Q,
    ) -> Result<Vec<Q::Row>, Error> {
        let mut statement = rusqlite::Connection::prepare(self, &query.sql_text())
            .map_err(|e| Error::Database(e.to_string()))?;

        let mut rows = statement.query([])
            .map_err(|e| Error::Database(e.to_string()))?;
        
        let mut result = vec![];
        while let Some(row) = rows.next().map_err(|e| Error::Database(e.to_string()))? {
            result.push(FromRow::from_row(row)?);
        }

        Ok(result)
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
        PostIndexed: for<'a> FromRow<C::Row<'a>>,
    {
        type Row = PostIndexed;

        fn to_row(&self) -> Vec<C::Param<'_>> {
            //vec![&self.0]
            todo!()
        }
    }
}
