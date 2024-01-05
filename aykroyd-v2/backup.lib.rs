pub enum Error {
    FromSql(String),
}

pub trait FromSql<Row>: Sized {
    fn get_index(row: Row, index: usize) -> Result<Self, Error>;
    fn get_name(row: Row, name: &str) -> Result<Self, Error>;
}

impl<T: mysql::prelude::FromValue> FromSql<&mysql::Row> for T {
    fn get_index(row: &mysql::Row, index: usize) -> Result<Self, Error> {
        row.get_opt(index)
            .ok_or_else(|| Error::FromSql(format!("unknown column {}", index)))?
            .map_err(|e| Error::FromSql(e.to_string()))
    }

    fn get_name(row: &mysql::Row, name: &str) -> Result<Self, Error> {
        row.get_opt(name)
            .ok_or_else(|| Error::FromSql(format!("unknown column {}", name)))?
            .map_err(|e| Error::FromSql(e.to_string()))
    }
}

impl<'a, T: rusqlite::types::FromSql> FromSql<&rusqlite::Row<'a>> for T {
    fn get_index(row: &rusqlite::Row, index: usize) -> Result<Self, Error> {
        row.get(index).map_err(|e| Error::FromSql(e.to_string()))
    }

    fn get_name(row: &rusqlite::Row, name: &str) -> Result<Self, Error> {
        row.get(name).map_err(|e| Error::FromSql(e.to_string()))
    }
}

impl<'a, T: tokio_postgres::types::FromSql<'a>> FromSql<&'a tokio_postgres::Row> for T {
    fn get_index(row: &'a tokio_postgres::Row, index: usize) -> Result<Self, Error> {
        row.try_get(index).map_err(|e| Error::FromSql(e.to_string()))
    }

    fn get_name(row: &'a tokio_postgres::Row, name: &str) -> Result<Self, Error> {
        row.try_get(name).map_err(|e| Error::FromSql(e.to_string()))
    }
}

pub struct ColumnsIndexed<Row> {
    row: Row,
    offset: usize,
}

impl<Row> ColumnsIndexed<Row> {
    fn new(row: Row) -> Self {
        ColumnsIndexed {
            row,
            offset: 0,
        }
    }
}

pub struct ColumnsNamed<Row> {
    row: Row,
    prefix: String,
}

impl<Row> ColumnsNamed<Row> {
    fn new(row: Row) -> Self {
        ColumnsNamed {
            row,
            prefix: String::new(),
        }
    }
}

pub trait FromColumnsIndexed<Row>: Sized {
    fn from_columns(columns: &ColumnsIndexed<Row>) -> Result<Self, Error>;
}

pub trait FromColumnsNamed<Row>: Sized {
    fn from_columns(columns: &ColumnsNamed<Row>) -> Result<Self, Error>;
}

pub trait FromRow<Row>: Sized {
    fn from_row(row: Row) -> Result<Self, Error>;
}

impl<Row, T: FromColumnsIndexed<Row>> FromRow<Row> for T {
    fn from_row(row: Row) -> Result<Self, Error> {
        let columns = ColumnsIndexed::new(row);
        FromColumnsIndexed::from_columns(&columns)
    }
}
