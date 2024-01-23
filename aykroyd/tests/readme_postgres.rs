#![cfg(feature = "postgres")]
#![allow(dead_code)]

use aykroyd::postgres::{Client, Error};
use aykroyd::{FromRow, Query, Statement};
use postgres::NoTls;

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
#[aykroyd(
    row(Pet),
    text = "
    SELECT id, name, species FROM pets
"
)]
struct GetAllPets;

fn main() -> Result<(), Error> {
    // Connect to the database
    let mut client = Client::connect("host=localhost user=postgres", NoTls)?;

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
