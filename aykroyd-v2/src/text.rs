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
