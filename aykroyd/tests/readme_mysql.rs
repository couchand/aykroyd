#![cfg(feature = "mysql")]
#![allow(dead_code)]

use aykroyd::mysql::{Client, Error};
use aykroyd::{FromRow, Query, Statement};

#[derive(Statement)]
#[aykroyd(text = "
    INSERT INTO pets (name, species) VALUES (?, ?)
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
    let mut client = Client::new("mysql://user:password@locahost:3307/db_name")?;

    // Execute a statement, returning the number of rows modified.
    let insert_count = client.execute(&InsertPet {
        name: "Dan",
        species: "Felis maria",
    })?;
    assert_eq!(insert_count, 1);

    // Run a query and map the result objects.
    let rows = client.query(&GetAllPets)?;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, "Dan");

    Ok(())
}
