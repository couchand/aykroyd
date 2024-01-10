use ::aykroyd_v2::{FromRow, Query, Statement};
use ::aykroyd_v2::row::{FromColumnsIndexed, FromColumnsNamed};

#[derive(Query)]
#[aykroyd(row((String, Option<String>)), query = "SELECT name, email FROM user")]
pub struct GetAllUsersAsTuple;

#[derive(Statement)]
#[aykroyd(query = "INSERT INTO user (name, email) VALUES ($1, $2)")]
pub struct InsertUser<'a> {
    pub name: &'a str,
    pub email: &'a str,
}

#[derive(Debug, FromColumnsIndexed, FromColumnsNamed)]
pub struct User {
    pub name: String,
    pub email: Option<String>,
}

#[derive(FromColumnsIndexed, FromColumnsNamed)]
pub struct Post {
    pub title: String,
    pub body: Option<String>,
}

#[derive(FromColumnsIndexed)]
pub struct PostWithAuthorIndexed {
    #[aykroyd(nested)]
    pub post: Post,
    #[aykroyd(nested)]
    pub user: User,
}

#[derive(FromColumnsNamed)]
pub struct PostWithAuthorNamed {
    #[aykroyd(nested)]
    pub post: Post,
    #[aykroyd(nested)]
    pub user: User,
}

#[derive(Debug, FromRow)]
#[aykroyd(by_index)]
pub struct AuthoredPostIndexed {
    pub title: String,
    #[aykroyd(nested)]
    pub author: User,
    pub body: Option<String>,
    #[aykroyd(nested)]
    pub editor: User,
}

#[derive(Query)]
#[aykroyd(
    row(AuthoredPostIndexed),
    query = "\
        SELECT \
            post.title, \
            author.name, author.email, \
            post.body, \
            editor.name, editor.email \
        FROM post \
        INNER JOIN user author on post.author_id = user.id \
        INNER JOIN user editor on post.editor_id = user.id \
        WHERE id = $1
    "
)]
pub struct QueryPostsByIdIndexed(isize);

#[derive(Debug, FromRow)]
#[aykroyd(by_name)]
pub struct AuthoredPostNamed {
    pub title: String,
    #[aykroyd(nested)]
    pub author: User,
    pub body: Option<String>,
    #[aykroyd(nested)]
    pub editor: User,
}

#[derive(Query)]
#[aykroyd(
    row(AuthoredPostNamed),
    query = "\
        SELECT \
            post.title, \
            author.name author_name, author.email author_email, \
            post.body, \
            editor.name editor_name, editor.email editor_email \
        FROM post \
        INNER JOIN user author on post.author_id = user.id \
        INNER JOIN user editor on post.editor_id = user.id \
        WHERE id = $1
    "
)]
pub struct QueryPostsByIdNamed(isize);

pub fn query_mysql() {
    let url = "mysql://root:password@localhost:3307/db_name";
    let mut client = ::aykroyd_v2::mysql::Client::new(url).unwrap();
    println!("{:?}", client.query(&GetAllUsersAsTuple));
    println!("{:?}", client.query(&QueryPostsByIdIndexed(1)));
    println!("{:?}", client.query(&QueryPostsByIdNamed(2)));
}
