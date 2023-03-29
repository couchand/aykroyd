#[cfg(feature = "derive")]
pub use akroyd_derive::*;

mod traits;
pub use traits::*;

#[cfg(feature = "async")]
mod async_client;
#[cfg(feature = "async")]
pub use async_client::*;

#[cfg(feature = "sync")]
mod sync_client;
#[cfg(feature = "sync")]
pub use sync_client::*;

#[cfg(any(feature = "async", feature = "sync"))]
type StatementKey = String; // TODO: more

#[doc(hidden)]
pub mod types {
    pub use tokio_postgres::types::ToSql;
    pub use tokio_postgres::{Error, Row};
}
