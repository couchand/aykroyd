# aykroyd: An opinionated micro-ORM for PostgreSQL.

All database queries are represented by a plain Rust struct that implements
either `Statement` or `Query` (and maybe `QueryOne`).  These trait implementations
logically binds the SQL text to the types of the inputs, and the `Query` or
`QueryOne` implementation does the same for the output rows.  This lets the
struct act as an event-sourcing layer between your application and the database.

The binding is not magic; there is no compile-time verification against a
live database.  It is instead an assertion by the developer, one that you
would be wise to verify.  To do so, you'll need to connect to a live database.
It is recommended to do so in a suite of automated tests which can be run
against any database environment to verify that particular tier.

Once you have a `Statement` or `Query` in hand, you'll need a database
connection to run it.  Aykroyd supports the following database client crates:

| DB | Backend Crate | Feature | Sync/Async | Client |
| -- | ------------- | ------- | ---------- | ------ |
| PostgreSQL | [postgres](https://crates.io/crates/postgres) | `postgres` | Sync | `aykroyd::postgres::Client` |
| PostgreSQL | [tokio-postgres](https://crates.io/crates/tokio-postgres) | `tokio-postgres` | Async | `aykroyd::tokio_postgres::Client` |
| MySQL/MariaDB | [mysql](https://crates.io/crates/mysql) | `mysql` | Sync | `aykroyd::mysql::Client` |
| SQLite | [rusqlite](https://crates.io/crates/rusqlite) | `rusqlite` | Sync | `aykroyd::rusqlite::Client` |

See [the documentation](https://docs.rs/aykroyd/latest/aykroyd/) for more details.

## Examples

### Synchronous

An example of the synchronous PostgreSQL client.

```rust
use postgres::{NoTls, Error};
use aykroyd::postgres::Client;

#[derive(aykroyd::FromRow)]
struct Customer {
    id: i32,
    first_name: String,
    last_name: String,
}

#[derive(aykroyd::Statement)]
#[aykroyd(text = "INSERT INTO customers (first_name, last_name) VALUES ($1, $2)")]
struct InsertCustomer<'a> {
    first_name: &'a str,
    last_name: &'a str,
}

#[derive(aykroyd::Query)]
#[aykroyd(row(Customer), text = "SELECT id, first_name, last_name FROM customers")]
struct GetAllCustomers;

fn try_main() -> Result<(), Error> {
    // Connect to the database
    let mut client =
        Client::connect("host=localhost user=postgres", NoTls)?;

    // Execute a statement, returning the number of rows modified.
    let insert_count = client.execute(&InsertCustomer {
        first_name: "Dan",
        last_name: "Aykroyd",
    })?;
    assert_eq!(insert_count, 1);

    // Run a query and map the result objects.
    let rows = client.query(&GetAllCustomers)?;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].first_name, "Dan");

    Ok(())
}
```

### Asynchronous

An example of the asynchronous PostgreSQL client.

```rust
use tokio_postgres::{NoTls, Error};
use aykroyd::tokio_postgres::connect;

#[derive(aykroyd::FromRow)]
struct Customer {
    id: i32,
    first_name: String,
    last_name: String,
}

#[derive(aykroyd::Statement)]
#[aykroyd(text = "INSERT INTO customers (first_name, last_name) VALUES ($1, $2)")]
struct InsertCustomer<'a> {
    first_name: &'a str,
    last_name: &'a str,
}

#[derive(aykroyd::Query)]
#[aykroyd(row(Customer), text = "SELECT id, first_name, last_name FROM customers")]
struct GetAllCustomers;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Connect to the database
    let (mut client, conn) =
        connect("host=localhost user=postgres", NoTls).await?;

    // As with tokio_postgres, you need to spawn a task for the connection.
    tokio::spawn(async move {
        if let Err(e) = conn.await {
            eprintln!("connection error: {e}");
        }
    });

    // Execute a statement, returning the number of rows modified.
    let insert_count = client.execute(&InsertCustomer {
        first_name: "Dan",
        last_name: "Aykroyd",
    }).await?;
    assert_eq!(insert_count, 1);

    // Run a query and map the result objects.
    let rows = client.query(&GetAllCustomers).await?;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].first_name, "Dan");

    Ok(())
}
