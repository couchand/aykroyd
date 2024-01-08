use super::client::SyncClient;
use super::combinator::EitherQuery;
use super::query::{QueryText, ToParam, ToParams};
use super::row::{ColumnsIndexed, ColumnsNamed, FromColumn, FromColumnsIndexed, FromColumnsNamed};
use super::*;

struct FakeRow {
    columns: Vec<String>,
    tuple: Vec<String>,
}

impl FromColumn<&FakeRow, usize> for String {
    fn get(row: &FakeRow, index: usize) -> Result<String, Error> {
        row.tuple
            .get(index)
            .cloned()
            .ok_or(Error::FromColumn("not found".into()))
    }
}

impl FromColumn<&FakeRow, &str> for String {
    fn get(row: &FakeRow, name: &str) -> Result<String, Error> {
        row.columns
            .iter()
            .position(|d| d.eq_ignore_ascii_case(name))
            .and_then(|i| row.tuple.get(i))
            .cloned()
            .ok_or(Error::FromColumn("not found".into()))
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

impl<Row> FromColumnsIndexed<Row> for User
where
    String: for<'a> FromColumn<&'a Row, usize>,
{
    fn from_columns(columns: ColumnsIndexed<Row>) -> Result<Self, Error> {
        Ok(User {
            name: columns.get(0)?,
        })
    }
}

impl<Row> FromColumnsNamed<Row> for User
where
    String: for<'a, 'b> FromColumn<&'a Row, &'b str>,
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

impl<Row> FromRow<Row> for PostIndexed
where
    String: for<'a> FromColumn<&'a Row, usize>,
{
    fn from_row(row: &Row) -> Result<Self, Error> {
        FromColumnsIndexed::from_columns(ColumnsIndexed::new(row))
    }
}

impl<Row> FromColumnsIndexed<Row> for PostIndexed
where
    String: for<'a> FromColumn<&'a Row, usize>,
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
        columns: vec!["text".into(), "user_name".into()],
        tuple: vec!["my cool post!".into(), "Sam Author".into()],
    };
    let post = PostIndexed::from_row(&result).unwrap();
    assert_eq!("Sam Author", post.user.name);
    assert_eq!("my cool post!", post.text);
}

struct PostNamed {
    text: String,
    user: User,
}

impl<Row> FromRow<Row> for PostNamed
where
    String: for<'a, 'b> FromColumn<&'a Row, &'b str>,
{
    fn from_row(row: &Row) -> Result<Self, Error> {
        FromColumnsNamed::from_columns(ColumnsNamed::new(row))
    }
}

impl<Row> FromColumnsNamed<Row> for PostNamed
where
    String: for<'a, 'b> FromColumn<&'a Row, &'b str>,
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
        columns: vec!["text".into(), "user_name".into()],
        tuple: vec!["my cool post!".into(), "Sam Author".into()],
    };
    let post = PostNamed::from_row(&result).unwrap();
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
    let query: EitherQuery<GetAllPosts, GetActivePosts> = EitherQuery::Right(GetActivePosts);
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
    for<'a> PostIndexed: FromRow<C::Row<'a>>,
    Self: ToParams<C>,
{
    type Row = PostIndexed;
}

struct FakeClient(Vec<FakeRow>);

impl Client for FakeClient {
    type Row<'a> = FakeRow;
    type Param<'a> = String;
}

#[test]
fn smoke_to_params() {
    let query = GetPostsByUser("foobar".into());
    let row = <GetPostsByUser as ToParams<FakeClient>>::to_params(&query);
    assert_eq!(1, row.len());
    assert_eq!("foobar", row[0]);
}

impl SyncClient for FakeClient {
    fn query<Q: Query<Self>>(&mut self, _query: &Q) -> Result<Vec<Q::Row>, Error> {
        let mut rows = vec![];
        for row in &self.0 {
            rows.push(FromRow::from_row(row)?);
        }
        Ok(rows)
    }

    fn execute<S: Statement<Self>>(&mut self, statement: &S) -> Result<u64, Error> {
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

    fn prepare<S: StaticQueryText>(&mut self) -> Result<(), Error> {
        Ok(())
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

    assert!(matches!(result, Ok(_)));
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
