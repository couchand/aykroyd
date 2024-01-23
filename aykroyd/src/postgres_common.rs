pub mod params_iter {
    pub struct ParamsIter<'a>(Vec<&'a (dyn tokio_postgres::types::ToSql + Sync)>);

    impl<'a> ParamsIter<'a> {
        pub fn from_params(
            params: Option<Vec<&'a (dyn tokio_postgres::types::ToSql + Sync)>>,
        ) -> Self {
            let params = match params {
                None => vec![],
                Some(mut params) => {
                    params.reverse();
                    params
                }
            };
            ParamsIter(params)
        }
    }

    impl<'a> std::iter::Iterator for ParamsIter<'a> {
        type Item = &'a (dyn tokio_postgres::types::ToSql + Sync);

        fn next(&mut self) -> Option<&'a (dyn tokio_postgres::types::ToSql + Sync)> {
            self.0.pop()
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            (self.0.len(), Some(self.0.len()))
        }
    }

    impl<'a> std::iter::ExactSizeIterator for ParamsIter<'a> {
    }
}

#[macro_export]
macro_rules! postgres_client {
    (
        $client:ty
    ) => {
        /// The type of errors from a `Client`.
        pub type Error = error::Error<tokio_postgres::Error>;

        impl<T> FromColumnIndexed<$client> for T
        where
            T: tokio_postgres::types::FromSqlOwned,
        {
            fn from_column(row: &tokio_postgres::Row, index: usize) -> Result<Self, Error> {
                row.try_get(index).map_err(Error::from_column)
            }
        }

        impl<T> FromColumnNamed<$client> for T
        where
            T: tokio_postgres::types::FromSqlOwned,
        {
            fn from_column(row: &tokio_postgres::Row, name: &str) -> Result<Self, Error> {
                row.try_get(name).map_err(Error::from_column)
            }
        }

        impl<T> ToParam<$client> for T
        where
            T: tokio_postgres::types::ToSql + Sync,
        {
            fn to_param(&self) -> &(dyn tokio_postgres::types::ToSql + Sync) {
                self
            }
        }

        impl crate::client::Client for $client {
            type Row<'a> = tokio_postgres::Row;
            type Param<'a> = &'a (dyn tokio_postgres::types::ToSql + Sync);
            type Error = tokio_postgres::Error;
        }
    };
}
