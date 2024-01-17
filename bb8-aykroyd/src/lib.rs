//! Aykroyd support for the `bb8` connection pool.
#![deny(missing_docs, missing_debug_implementations)]

#[cfg(feature = "tokio-postgres")]
pub mod tokio_postgres;
