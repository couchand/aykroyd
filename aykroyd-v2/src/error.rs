#[derive(Debug)]
pub enum Error {
    FromSql(String),
    Query(String),
    Prepare(String),
}
