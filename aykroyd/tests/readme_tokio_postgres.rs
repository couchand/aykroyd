#![cfg(feature = "tokio-postgres")]

use tokio_postgres::NoTls;
use aykroyd::tokio_postgres::{connect, Client, Error};
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
