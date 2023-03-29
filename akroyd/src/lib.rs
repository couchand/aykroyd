//! A Rust micro-ORM for PostgreSQL.
//!
//! All database queries are represented by a plain Rust struct that implements
//! [`Statement`](./trait.Statement.html) and possibly one of [`Query`](./trait.Query.html)
//! or [`QueryOne`](./trait.QueryOne.html).  The `Statement` implementation
//! logically binds the SQL text to the types of the inputs, and the `Query` or
//! `QueryOne` implementation does the same for the output rows.  This lets the
//! struct act as an event-sourcing layer between your application and the database.
//!
//! The binding is not magic; there is no compile-time verification against a
//! live database.  It is instead an assertion by the developer, one that you
//! would be wise to verify.  To do so, you'll need to connect to a live database.
//! It is recommended to do so in a suite of automated tests which can be run
//! against any database environment to verify that particular tier.
#![cfg_attr(
    any(feature = "sync", feature = "async"),
    doc = r##"

# Example
"##
)]
#![cfg_attr(
    feature = "sync",
    doc = r##"

An example of the synchronous client.

```no_run
use postgres::{NoTls, Error};

#[derive(akroyd::FromRow)]
struct Customer {
    id: i32,
    first_name: String,
    last_name: String,
}

#[derive(akroyd::Statement)]
#[query(text = "INSERT INTO customers (first_name, last_name) VALUES ($1, $2)")]
struct InsertCustomer<'a> {
    first_name: &'a str,
    last_name: &'a str,
}

#[derive(akroyd::Query)]
#[query(row(Customer), text = "SELECT id, first_name, last_name FROM customers")]
struct GetAllCustomers;

fn try_main() -> Result<(), Error> {
    // Connect to the database
    let mut client =
        akroyd::Client::connect("host=localhost user=postgres", NoTls)?;

    // Execute a statement, returning the number of rows modified.
    let insert_count = client.execute(&InsertCustomer {
        first_name: "Dan",
        last_name: "Aykroyd", // two ys??
    })?;
    assert_eq!(insert_count, 1);

    // Run a query and map the result objects.
    let rows = client.query(&GetAllCustomers)?;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].first_name, "Dan");

    Ok(())
}
```
"##
)]
#![cfg_attr(
    feature = "async",
    doc = r##"

An example of the asynchronous client.

```no_run
use tokio_postgres::{NoTls, Error};

#[derive(akroyd::FromRow)]
struct Customer {
    id: i32,
    first_name: String,
    last_name: String,
}

#[derive(akroyd::Statement)]
#[query(text = "INSERT INTO customers (first_name, last_name) VALUES ($1, $2)")]
struct InsertCustomer<'a> {
    first_name: &'a str,
    last_name: &'a str,
}

#[derive(akroyd::Query)]
#[query(row(Customer), text = "SELECT id, first_name, last_name FROM customers")]
struct GetAllCustomers;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Connect to the database
    let (mut client, conn) =
        akroyd::connect("host=localhost user=postgres", NoTls).await?;

    // As with tokio_postgres, you need to spawn a task for the connection.
    tokio::spawn(async move {
        if let Err(e) = conn.await {
            eprintln!("connection error: {e}");
        }
    });

    // Execute a statement, returning the number of rows modified.
    let insert_count = client.execute(&InsertCustomer {
        first_name: "Dan",
        last_name: "Aykroyd", // two ys??
    }).await?;
    assert_eq!(insert_count, 1);

    // Run a query and map the result objects.
    let rows = client.query(&GetAllCustomers).await?;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].first_name, "Dan");

    Ok(())
}
```
"##
)]

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
