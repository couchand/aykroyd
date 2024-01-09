use crate::client::{Client, FromColumnIndexed, FromColumnNamed, SyncClient, SyncTransaction, ToParam};
use crate::combinator::Either;
use crate::error::Error;
use crate::query::{QueryText, StaticQueryText, ToParams};
use crate::row::{ColumnsIndexed, ColumnsNamed, FromColumnsIndexed, FromColumnsNamed};
use crate::*;

struct FakeRow {
    columns: Vec<String>,
    tuple: Vec<String>,
}

impl FromColumnIndexed<FakeClient> for String {
    fn from_column(row: &FakeRow, index: usize) -> Result<String, Error<String>> {
        row.tuple
            .get(index)
            .cloned()
            .ok_or(Error::from_column("not found".into()))
    }
}

impl FromColumnNamed<FakeClient> for String {
    fn from_column(row: &FakeRow, name: &str) -> Result<String, Error<String>> {
        row.columns
            .iter()
            .position(|d| d.eq_ignore_ascii_case(name))
            .and_then(|i| row.tuple.get(i))
            .cloned()
            .ok_or(Error::from_column("not found".into()))
    }
}

impl ToParam<FakeClient> for String {
    fn to_param(&self) -> String {
        self.clone()
    }
}

struct User {
    name: String,
}

impl<C: Client> FromColumnsIndexed<C> for User
where
    String: FromColumnIndexed<C>,
{
    fn from_columns(columns: ColumnsIndexed<C>) -> Result<Self, Error<C::Error>> {
        Ok(User {
            name: columns.get(0)?,
        })
    }
}

impl<C: Client> FromColumnsNamed<C> for User
where
    String: FromColumnNamed<C>,
{
    fn from_columns(columns: ColumnsNamed<C>) -> Result<Self, Error<C::Error>> {
        Ok(User {
            name: columns.get("name")?,
        })
    }
}

struct PostIndexed {
    text: String,
    user: User,
}

impl<C: Client> FromRow<C> for PostIndexed
where
    String: FromColumnIndexed<C>,
{
    fn from_row(row: &C::Row<'_>) -> Result<Self, Error<C::Error>> {
        FromColumnsIndexed::from_columns(ColumnsIndexed::new(row))
    }
}

impl<C: Client> FromColumnsIndexed<C> for PostIndexed
where
    String: FromColumnIndexed<C>,
    User: FromColumnsIndexed<C>,
{
    fn from_columns(columns: ColumnsIndexed<C>) -> Result<Self, Error<C::Error>> {
        Ok(PostIndexed {
            text: columns.get(0)?,
            user: columns.get_nested(1)?,
        })
    }
}

#[test]
fn smoke_indexed() {
    let result = FakeRow {
        columns: vec!["text".into(), "user_name".into()],
        tuple: vec!["my cool post!".into(), "Sam Author".into()],
    };
    let post = <PostIndexed as FromRow<FakeClient>>::from_row(&result).unwrap();
    assert_eq!("Sam Author", post.user.name);
    assert_eq!("my cool post!", post.text);
}

struct PostNamed {
    text: String,
    user: User,
}

impl<C: Client> FromRow<C> for PostNamed
where
    String: FromColumnNamed<C>,
{
    fn from_row(row: &C::Row<'_>) -> Result<Self, Error<C::Error>> {
        FromColumnsNamed::from_columns(ColumnsNamed::new(row))
    }
}

impl<C: Client> FromColumnsNamed<C> for PostNamed
where
    String: FromColumnNamed<C>,
    User: FromColumnsNamed<C>,
{
    fn from_columns(columns: ColumnsNamed<C>) -> Result<Self, Error<C::Error>> {
        Ok(PostNamed {
            text: columns.get("text")?,
            user: columns.get_nested("user_")?,
        })
    }
}

#[test]
fn smoke_named() {
    let result = FakeRow {
        columns: vec!["text".into(), "user_name".into()],
        tuple: vec!["my cool post!".into(), "Sam Author".into()],
    };
    let post = <PostNamed as FromRow<FakeClient>>::from_row(&result).unwrap();
    assert_eq!("Sam Author", post.user.name);
    assert_eq!("my cool post!", post.text);
}

struct GetAllPosts;

impl StaticQueryText for GetAllPosts {
    const QUERY_TEXT: &'static str = "SELECT text, user.name user_name FROM post";
}

#[test]
fn smoke_static_text() {
    let query = GetAllPosts;
    assert_eq!(GetAllPosts::QUERY_TEXT, query.query_text());
}

struct GetActivePosts;

impl StaticQueryText for GetActivePosts {
    const QUERY_TEXT: &'static str = "SELECT text, user.name user_name FROM post WHERE active";
}

#[test]
fn smoke_dynamic_text() {
    let query: Either<GetAllPosts, GetActivePosts> = Either::Right(GetActivePosts);
    assert_eq!(GetActivePosts::QUERY_TEXT, query.query_text());
}

struct GetPostsByUser(String);

impl StaticQueryText for GetPostsByUser {
    const QUERY_TEXT: &'static str = "SELECT text, user.name user_name FROM post \
        WHERE user.name = $1";
}

impl<C: Client> ToParams<C> for GetPostsByUser
where
    String: ToParam<C>,
{
    fn to_params(&self) -> Vec<C::Param<'_>> {
        vec![self.0.to_param()]
    }
}

impl<C: Client> Query<C> for GetPostsByUser
where
    for<'a> PostIndexed: FromRow<C>,
    Self: ToParams<C>,
{
    type Row = PostIndexed;
}

struct FakeClient(Vec<FakeRow>);

impl Client for FakeClient {
    type Row<'a> = FakeRow;
    type Param<'a> = String;
    type Error = String;
}

#[test]
fn smoke_to_params() {
    let query = GetPostsByUser("foobar".into());
    let row = <GetPostsByUser as ToParams<FakeClient>>::to_params(&query);
    assert_eq!(1, row.len());
    assert_eq!("foobar", row[0]);
}

impl SyncClient for FakeClient {
    fn query<Q: Query<Self>>(&mut self, _query: &Q) -> Result<Vec<Q::Row>, Error<String>> {
        let mut rows = vec![];
        for row in &self.0 {
            rows.push(FromRow::from_row(row)?);
        }
        Ok(rows)
    }

    fn execute<S: Statement<Self>>(&mut self, statement: &S) -> Result<u64, Error<String>> {
        let params = statement.to_params();
        assert_eq!(1, params.len());
        let text = params.into_iter().next().unwrap();

        if self.0.is_empty() {
            self.0.push(FakeRow {
                columns: vec!["text".into(), "user_name".into()],
                tuple: vec![text, "Sam Author".into()],
            });
        } else {
            self.0[0].tuple[0] = text
        }
        Ok(1)
    }

    fn prepare<S: StaticQueryText>(&mut self) -> Result<(), Error<String>> {
        Ok(())
    }

    type Transaction<'a> = ();

    fn transaction(&mut self) -> Result<(), Error<String>> {
        Ok(())
    }
}

impl SyncTransaction<FakeClient> for () {
    fn commit(self) -> Result<(), Error<String>> {
        todo!()
    }

    fn rollback(self) -> Result<(), Error<String>> {
        todo!()
    }

    fn query<Q: Query<FakeClient>>(&mut self, _query: &Q) -> Result<Vec<Q::Row>, Error<String>> {
        todo!()
    }

    fn execute<S: Statement<FakeClient>>(&mut self, _statement: &S) -> Result<u64, Error<String>> {
        todo!()
    }

    fn prepare<S: StaticQueryText>(&mut self) -> Result<(), Error<String>> {
        todo!()
    }
}

#[test]
fn smoke_query() {
    let query = GetPostsByUser("Sam Author".into());
    let row = FakeRow {
        columns: vec!["text".into(), "user_name".into()],
        tuple: vec!["my cool post!".into(), "Sam Author".into()],
    };
    let mut client = FakeClient(vec![row]);

    let result = client.query(&query);

    assert!(result.is_ok());
    if let Ok(rows) = result {
        assert_eq!(1, rows.len());
        assert_eq!("Sam Author", rows[0].user.name);
        assert_eq!("my cool post!", rows[0].text);
    }
}

struct UpdatePost(String);

impl StaticQueryText for UpdatePost {
    const QUERY_TEXT: &'static str = "UPDATE post SET text = $1";
}

impl<C: Client> ToParams<C> for UpdatePost
where
    String: ToParam<C>,
{
    fn to_params(&self) -> Vec<C::Param<'_>> {
        vec![self.0.to_param()]
    }
}

impl<C: Client> Statement<C> for UpdatePost where Self: ToParams<C> {}

#[test]
fn smoke_statement() {
    let statement = UpdatePost("i can change".into());
    let row = FakeRow {
        columns: vec!["text".into(), "user_name".into()],
        tuple: vec!["my cool post!".into(), "Sam Author".into()],
    };
    let mut client = FakeClient(vec![row]);

    let result = client.execute(&statement);

    assert!(matches!(result, Ok(1)));

    assert_eq!("i can change", client.0[0].tuple[0]);
}
