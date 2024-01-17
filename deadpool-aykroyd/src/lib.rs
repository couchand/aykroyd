//! Aykroyd support for the `deadpool` connection pool.
#![deny(missing_docs, missing_debug_implementations)]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "tokio-postgres")]
#[cfg_attr(docsrs, doc(cfg(feature = "tokio-postgres")))]
pub mod tokio_postgres;
