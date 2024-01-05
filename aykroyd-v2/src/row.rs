use super::Error;

pub trait FromSql<Row, Index>: Sized {
    fn get(row: Row, index: Index) -> Result<Self, Error>;
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
