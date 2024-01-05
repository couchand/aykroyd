pub trait QueryText {
    fn query_text(&self) -> String;
}

pub trait StaticQueryText {
    const QUERY_TEXT: &'static str;
}

impl<S: StaticQueryText> QueryText for S {
    fn query_text(&self) -> String {
        Self::QUERY_TEXT.into()
    }
}
