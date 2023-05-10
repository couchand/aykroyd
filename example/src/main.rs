#[cfg(feature = "sync")]
use akroyd::sync_client::Client;
use akroyd_migrate::*;

static MIGRATIONS: embedded::EmbeddedRepo = include_migrations!();

#[cfg(feature = "sync")]
fn main() {
    try_main().unwrap()
}

#[cfg(all(feature = "full", feature = "sync"))]
fn try_main() -> Result<(), Error> {
    println!("Loading embedded migrations...");
    let mut local_repo = MIGRATIONS.load();
    println!("{local_repo:?}");

    let mut client = Client::connect(
        "host=localhost user=akroyd_test password=akroyd_test",
        tokio_postgres::NoTls,
    )?;

    println!("Loading from database...");
    let mut db_repo = db::DatabaseRepo::from_client(&mut client).unwrap();
    println!("{db_repo:?}");

    let plan = plan::Plan::from_db_and_local(&mut db_repo, &mut local_repo)?;
    println!("Plan: {plan:?}");

    if !plan.is_empty() {
        if !plan.is_fast_forward() {
            loop {
                println!("Plan is not a fast-forward merge, continue (y/n)?");

                let mut line = String::new();
                std::io::stdin().read_line(&mut line).unwrap();
                match line.trim() {
                    "y" | "Y" => break,
                    "n" | "N" => {
                        eprintln!("Refusing to apply non-fast-forward plan.");
                        std::process::exit(-1);
                    }
                    _ => {}
                }
            }
        }

        println!("Applying....");

        db_repo.apply(&plan).unwrap();

        println!("Done.");
    } else {
        println!("Nothing to do.");
    }

    Ok(())
}

#[cfg(all(feature = "lite", feature = "sync"))]
fn try_main() -> Result<(), Error> {
    let mut client = Client::connect(
        "host=localhost user=akroyd_test password=akroyd_test",
        tokio_postgres::NoTls,
    )?;

    println!("Migrating database...");
    match db::fast_forward_migrate(&mut client, MIGRATIONS.load()).unwrap() {
        akroyd_migrate::db::MergeStatus::NothingToDo => println!("Nothing to do."),
        akroyd_migrate::db::MergeStatus::Done => println!("Done."),
    }

    Ok(())
}
