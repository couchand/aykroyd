use ::aykroyd_v2::{FromRow, Query};
use ::aykroyd_v2::row::{FromColumnsIndexed, FromColumnsNamed};

#[derive(FromColumnsIndexed, FromColumnsNamed)]
struct User {
    name: String,
    email: Option<String>,
}

#[derive(FromColumnsIndexed, FromColumnsNamed)]
struct Post {
    title: String,
    body: Option<String>,
}

#[derive(FromColumnsIndexed)]
struct PostWithAuthorIndexed {
    #[aykroyd(nested)]
    post: Post,
    #[aykroyd(nested)]
    user: User,
}

#[derive(FromColumnsNamed)]
struct PostWithAuthorNamed {
    #[aykroyd(nested)]
    post: Post,
    #[aykroyd(nested)]
    user: User,
}

#[derive(FromRow)]
#[aykroyd(by_index)]
struct AuthoredPostIndexed {
    title: String,
    #[aykroyd(nested)]
    author: User,
    body: Option<String>,
    #[aykroyd(nested)]
    editor: User,
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
struct QueryPostsByIdIndexed(isize);

#[derive(FromRow)]
#[aykroyd(by_name)]
struct AuthoredPostNamed {
    title: String,
    #[aykroyd(nested)]
    author: User,
    body: Option<String>,
    #[aykroyd(nested)]
    editor: User,
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
struct QueryPostsByIdNamed(isize);
