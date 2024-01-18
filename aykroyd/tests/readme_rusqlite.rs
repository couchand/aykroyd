use aykroyd::rusqlite::{Client, Error};
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

fn main() -> Result<(), Error> {
    // Connect to the database
    let mut client = Client::open("./my_db.db3")?;

    // Execute a statement, returning the number of rows modified.
    let insert_count = client.execute(&InsertPet {
        name: "Dan",
        species: "Felis localis",
    })?;
    assert_eq!(insert_count, 1);

    // Run a query and map the result objects.
    let rows = client.query(&GetAllPets)?;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, "Dan");

    Ok(())
}
