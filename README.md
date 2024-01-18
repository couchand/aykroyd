# aykroyd: Zero-overhead ergonomic data access for Rust.

Aykroyd is a micro-ORM focused on developer ergonomics, with an
uncompromising commitment to performance.  Your database doesn't
have to be kept hidden behind abstraction layers or buried in
repetitive boilerplate anymore.

Database queries are represented by a plain Rust struct that implements
either `Statement` or `Query` (and maybe `QueryOne`).  The traits
`Statement` and `Query` share two common parent traits:

* `QueryText`, which gives access to the text of the
  query, and
* `ToParams`, which we can use to turn the struct into
  database parameters.

Using these together, a database client can prepare the text of a query
and then run it on a database, passing in the required parameters.

In addition, the `Query` trait has an associated type `Row` which must
implement:

* `FromRow`, to be deserialized from database rows.

All of these traits can be derived automatically, so the usual
gnarly database access code is reduced to simple struct definitions.
These structs logically bind the query text to input parameters and
output row types.

The binding is not magic, there is no verification against a database.
`Query` and `Statement` implementations are an assertion by the developer,
one that you would be wise to verify.  It is recommended to write a
suite of automated tests which can be run against any database tier.

```rust
use aykroyd::{FromRow, Query, Statement};

#[derive(Statement)]
#[aykroyd(text = "
    INSERT INTO pets (name, species) VALUES ($1, $2)
")]
struct InsertPet<'a> {
    name: &'a str,
    species: &'a str,
}

#[derive(FromRow)]
struct Pet {
    id: i32,
    name: String,
    species: String,
}

#[derive(Query)]
#[aykroyd(row(Pet), text = "
    SELECT id, name, species FROM pets
")]
struct GetAllPets;
```

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

Here's how it might look end-to-end with various clients.

### Synchronous

An example of the synchronous PostgreSQL client, available when compiled
with crate feature `postgres`.

```rust
use postgres::NoTls;
use aykroyd::postgres::Client;

fn try_main() -> Result<(), Error> {
    // Connect to the database
    let mut client =
        Client::connect("host=localhost user=postgres", NoTls)?;

    // Execute a statement, returning the number of rows modified.
    let insert_count = client.execute(&InsertPet {
        name: "Dan",
        species: "Felis synchronous",
    })?;
    assert_eq!(insert_count, 1);

    // Run a query and map the result objects.
    let rows = client.query(&GetAllPets)?;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, "Dan");

    Ok(())
}
```

### Asynchronous

An example of the asynchronous PostgreSQL client, available when compiled
with crate feature `tokio-postgres`.

```rust
use tokio_postgres::NoTls;
use aykroyd::tokio_postgres::{connect, Client};

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
    let insert_count = client.execute(&InsertPet {
        name: "Dan",
        species: "Felis asynchronous",
    }).await?;
    assert_eq!(insert_count, 1);

    // Run a query and map the result objects.
    let rows = client.query(&GetAllPets).await?;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, "Dan");

    Ok(())
}
```

### More Details

See the example project directory `aykroyd-example` for more details.

## Contributing

We'd be lucky to have your help.  Join the
[mailing list](https://lists.sr.ht/~couch/aykroyd-dev) and say hello,
take a look at the [issue tracker](https://todo.sr.ht/~couch/aykroyd)
to see if anything looks interesting.  Please see the `CONTRIBUTING.md`
file for more information.
