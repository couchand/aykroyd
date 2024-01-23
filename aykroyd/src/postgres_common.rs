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
