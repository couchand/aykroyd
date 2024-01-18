use crate::{client, error, query, FromRow, Query, QueryOne, Statement};

#[derive(Debug, Default, Clone)]
pub struct TestClient {
    prepare_results: Vec<Result<()>>,
    query_results: Vec<Result<Vec<RowInner>>>,
    query_opt_results: Vec<Result<Option<RowInner>>>,
    query_one_results: Vec<Result<RowInner>>,
    execute_results: Vec<Result<u64>>,
    transaction_results: Vec<Result<()>>,
    commit_results: Vec<Result<()>>,
    rollback_results: Vec<Result<()>>,
    records: Vec<Record>,
}

impl TestClient {
    pub fn new() -> Self {
        TestClient::default()
    }

    pub fn row(&mut self, row: RowInner) -> Row<'_> {
        let statement = TestStatement::new(self);
        statement.execute_one(row)
    }

    pub fn push_query_result(&mut self, result: Result<Vec<RowInner>>) {
        self.query_results.push(result);
    }

    pub fn push_query_opt_result(&mut self, result: Result<Option<RowInner>>) {
        self.query_opt_results.push(result);
    }

    pub fn push_query_one_result(&mut self, result: Result<RowInner>) {
        self.query_one_results.push(result);
    }

    pub fn records(&self) -> &[Record] {
        &self.records[..]
    }
}

#[derive(Debug, Clone, Copy)]
struct TestStatement<'a>(core::marker::PhantomData<&'a ()>);

impl<'a> TestStatement<'a> {
    fn new<T>(_lifetime: &'a T) -> Self {
        TestStatement(core::marker::PhantomData)
    }

    fn execute_one(self, inner: RowInner) -> Row<'a> {
        Row(self, inner)
    }

    fn execute(self, inner: Vec<RowInner>) -> Vec<Row<'a>> {
        inner.into_iter().map(|inner| Row(self, inner)).collect()
    }
}

#[derive(Debug, Clone)]
pub struct Row<'a>(TestStatement<'a>, RowInner);

#[derive(Debug, Default, Clone)]
pub struct RowInner {
    pub names: Vec<String>,
    pub values: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Record {
    pub text: String,
    pub params: Option<Vec<String>>,
    pub kind: Kind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Prepare,
    Statement,
    Query,
    QueryOne,
    QueryOpt,
    Begin,
    Commit,
    Rollback,
}

pub trait ToParam {
    fn to_param(&self) -> String;
}

impl ToParam for &str {
    fn to_param(&self) -> String {
        self.to_string()
    }
}

impl<T: ToParam> client::ToParam<TestClient> for T {
    fn to_param(&self) -> &dyn ToParam {
        self
    }
}

impl client::FromColumnIndexed<TestClient> for String {
    fn from_column(row: &Row<'_>, index: usize) -> Result<Self> {
        Ok(row.1.values[index].clone()) // TODO: not panic
    }
}

impl client::FromColumnNamed<TestClient> for String {
    fn from_column(row: &Row<'_>, name: &str) -> Result<Self> {
        let index = row
            .1
            .names
            .iter()
            .enumerate()
            .find(|(_, n)| *n == name)
            .map(|(i, _)| i);
        Ok(row.1.values[index.unwrap()].clone()) // TODO: not panic
    }
}

impl client::FromColumnIndexed<TestClient> for i32 {
    fn from_column(row: &Row<'_>, index: usize) -> Result<Self> {
        Ok(row.1.values[index].parse().unwrap()) // TODO: not panic
    }
}

impl client::FromColumnNamed<TestClient> for i32 {
    fn from_column(row: &Row<'_>, name: &str) -> Result<Self> {
        let index = row
            .1
            .names
            .iter()
            .enumerate()
            .find(|(_, n)| *n == name)
            .map(|(i, _)| i);
        Ok(row.1.values[index.unwrap()].parse().unwrap()) // TODO: not panic
    }
}

#[derive(Debug, Default, Clone)]
pub struct ErrorDetails {
    pub message: String,
}

pub type Error = error::Error<ErrorDetails>;
pub type Result<T> = std::result::Result<T, Error>;

impl client::Client for TestClient {
    type Row<'a> = Row<'a>;
    type Param<'a> = &'a dyn ToParam;
    type Error = ErrorDetails;
}

impl TestClient {
    pub fn prepare<S: query::StaticQueryText>(&mut self) -> Result<()> {
        self.records.push(Record {
            text: S::QUERY_TEXT.into(),
            params: None,
            kind: Kind::Prepare,
        });
        self.prepare_results.pop().unwrap_or(Ok(()))
    }

    pub fn query<Q: Query<Self>>(&mut self, query: &Q) -> Result<Vec<Q::Row>> {
        self.records.push(Record {
            text: query.query_text(),
            params: query
                .to_params()
                .map(|params| params.into_iter().map(ToParam::to_param).collect()),
            kind: Kind::Query,
        });
        self.query_results
            .pop()
            .unwrap_or_else(|| Ok(vec![]))
            .and_then(|rows| {
                let statement = TestStatement::new(self);
                FromRow::from_rows(&statement.execute(rows))
            })
    }

    pub fn query_opt<Q: QueryOne<Self>>(&mut self, query: &Q) -> Result<Option<Q::Row>> {
        self.records.push(Record {
            text: query.query_text(),
            params: query
                .to_params()
                .map(|params| params.into_iter().map(ToParam::to_param).collect()),
            kind: Kind::QueryOpt,
        });
        self.query_opt_results
            .pop()
            .transpose()
            .and_then(|maybe_maybe_row| {
                let statement = TestStatement::new(self);
                Ok(match maybe_maybe_row {
                    Some(Some(row)) => Some(FromRow::from_row(&statement.execute_one(row))?),
                    _ => None,
                })
            })
    }

    pub fn query_one<Q: QueryOne<Self>>(&mut self, query: &Q) -> Result<Q::Row> {
        self.records.push(Record {
            text: query.query_text(),
            params: query
                .to_params()
                .map(|params| params.into_iter().map(ToParam::to_param).collect()),
            kind: Kind::QueryOne,
        });
        self.query_one_results.pop().unwrap().and_then(|row| {
            let statement = TestStatement::new(self);
            FromRow::from_row(&statement.execute_one(row))
        })
    }

    pub fn execute<S: Statement<Self>>(&mut self, statement: &S) -> Result<u64> {
        self.records.push(Record {
            text: statement.query_text(),
            params: statement
                .to_params()
                .map(|params| params.into_iter().map(ToParam::to_param).collect()),
            kind: Kind::Statement,
        });
        self.execute_results.pop().unwrap_or(Ok(0))
    }

    pub fn transaction(&mut self) -> Result<Transaction> {
        self.records.push(Record {
            text: "BEGIN".into(),
            params: None,
            kind: Kind::Begin,
        });
        if let Some(Err(e)) = self.transaction_results.pop() {
            return Err(e);
        }
        Ok(Transaction(self))
    }
}

#[derive(Debug)]
pub struct Transaction<'a>(&'a mut TestClient);

impl<'a> AsMut<TestClient> for Transaction<'a> {
    fn as_mut(&mut self) -> &mut TestClient {
        self.0
    }
}

impl<'a> Transaction<'a> {
    pub fn commit(mut self) -> Result<()> {
        self.as_mut().records.push(Record {
            text: "COMMIT".into(),
            params: None,
            kind: Kind::Commit,
        });
        self.as_mut().commit_results.pop().unwrap_or(Ok(()))
    }

    pub fn rollback(mut self) -> Result<()> {
        self.as_mut().records.push(Record {
            text: "ROLLBACK".into(),
            params: None,
            kind: Kind::Rollback,
        });
        self.as_mut().rollback_results.pop().unwrap_or(Ok(()))
    }

    pub fn prepare<S: query::StaticQueryText>(&mut self) -> Result<()> {
        self.as_mut().prepare::<S>()
    }

    pub fn query<Q: Query<TestClient>>(&mut self, query: &Q) -> Result<Vec<Q::Row>> {
        self.as_mut().query(query)
    }

    pub fn query_opt<Q: QueryOne<TestClient>>(&mut self, query: &Q) -> Result<Option<Q::Row>> {
        self.as_mut().query_opt(query)
    }

    pub fn query_one<Q: QueryOne<TestClient>>(&mut self, query: &Q) -> Result<Q::Row> {
        self.as_mut().query_one(query)
    }

    pub fn execute<S: Statement<TestClient>>(&mut self, statement: &S) -> Result<u64> {
        self.as_mut().execute(statement)
    }
}
