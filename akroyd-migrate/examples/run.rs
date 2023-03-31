use akroyd::sync_client::Client;
use akroyd_migrate::*;

fn main() {
    try_main().unwrap()
}

fn try_main() -> Result<(), tokio_postgres::Error> {
    let repo = local::LocalRepo::load("./migrations").unwrap();
    println!("{repo:?}");

    let mut client = Client::connect(
        "host=localhost user=akroyd_test password=akroyd_test",
        tokio_postgres::NoTls,
    )?;

    println!("Checking for existing migrations table...");
    match client.query_opt(&db::IsInsertable { table_name: "migrations" })? {
        Some((_, false)) => panic!("Migrations table exists, but user is unable to insert!"),
        Some((_, true)) => {}
        None => {
            println!("Creating table migrations...");
            client.execute(&db::CreateTableMigrations)?;
        }
    }

    println!("Starting transaction...");
    let mut txn = client.transaction()?;

    println!("Querying all migrations...");
    let migrations = txn.query(&db::AllMigrations)?;

    println!("{migrations:?}");

    println!("Committing transaction...");
    txn.commit()?;

    println!("Done.");
    Ok(())
}
