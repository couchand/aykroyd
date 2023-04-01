use akroyd::sync_client::Client;
use akroyd_migrate::*;

fn main() {
    try_main().unwrap()
}

fn try_main() -> Result<(), tokio_postgres::Error> {
    let mut local_repo = local::LocalRepo::load("./migrations").unwrap();
    println!("{local_repo:?}");

    let mut client = Client::connect(
        "host=localhost user=akroyd_test password=akroyd_test",
        tokio_postgres::NoTls,
    )?;

    println!("Checking for existing migration_text table...");
    match client.query_opt(&db::IsInsertable { table_name: "migration_text" })? {
        Some((_, false)) => panic!("Table migration_text exists, but user is unable to insert!"),
        Some((_, true)) => {}
        None => {
            println!("Creating table migration_text...");
            client.execute(&db::CreateTableMigrationText)?;
        }
    }

    println!("Checking for existing migration_commit table...");
    match client.query_opt(&db::IsInsertable { table_name: "migration_commit" })? {
        Some((_, false)) => panic!("Table migration_commit exists, but user is unable to insert!"),
        Some((_, true)) => {}
        None => {
            println!("Creating table migration_commit...");
            client.execute(&db::CreateTableMigrationCommit)?;
        }
    }

    println!("Checking for existing migration_dir enum...");
    match client.query_opt(&db::HasEnum { name: "migration_dir" })? {
        Some(_) => {}
        None => {
            println!("Creating enum migration_dir...");
            client.execute(&db::CreateEnumMigrationDir)?;
        }
    }

    println!("Checking for existing migration_current table...");
    match client.query_opt(&db::IsInsertable { table_name: "migration_current" })? {
        Some((_, false)) => panic!("Table migration_current exists, but user is unable to insert!"),
        Some((_, true)) => {}
        None => {
            println!("Creating table migration_current...");
            client.execute(&db::CreateTableMigrationCurrent)?;
        }
    }

    println!("Starting transaction...");
    let mut txn = client.transaction()?;

    println!("Querying all migrations...");
    let migrations = txn.query(&db::AllMigrations)?;
    println!("{migrations:?}");

    println!("Querying current db migration...");
    let current = txn.query(&db::AllCurrent)?;
    println!("{current:?}");

    let database_repo = db::DatabaseRepo::new(current, migrations);
    println!("{database_repo:?}");

/*
    for migration in migrations {
        match local_repo.take(&migration.commit_hash) {
            None => {
                println!("Database migration {} not found locally.", migration.commit_hash);
            }
            Some(local) => {
                println!("Database migration {} matched at {}.", migration.commit_hash, local.dir.display());
            }
        }
    }

    for migration in local_repo.iter() {
        println!("Local migration {} not in database.", migration.up_hash);

        println!("  Inserting it for testing purposes...");

        println!("  - inserting text {}", migration.up.hash);
        txn.execute(&db::InsertMigrationText {
            hash: &migration.up.hash,
            text: &migration.up.text,
        })?;

        println!("  - inserting commit {}", migration.up_hash);
        txn.execute(&db::InsertMigrationCommit {
            commit_hash: &migration.up_hash,
            name: &migration.dir.file_name().unwrap_or_default().to_str().unwrap_or_default(),
            deps: &migration.up_deps,
            text_hash: &migration.up.hash,
            created_on: chrono::Utc::now(),
        })?;

        println!("  - setting current to this");
        txn.execute(&db::SetCurrentMigration(db::Dir::Up, &migration.up_hash))?;
    }
    */

    println!("Committing transaction...");
    txn.commit()?;

    println!("Done.");
    Ok(())
}
