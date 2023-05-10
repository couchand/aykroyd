use akroyd::sync_client::Client;
use akroyd_migrate::*;

static MIGRATIONS: embedded::EmbeddedRepo = include_migrations!();

fn main() {
    try_main().unwrap()
}

#[cfg(feature = "full")]
fn try_main() -> Result<(), Error> {
    println!("Loading embedded migrations...");
    let mut local_repo = MIGRATIONS.load();
    println!("{local_repo:?}");

    let mut client = Client::connect(
        "host=localhost user=akroyd_test password=akroyd_test",
        tokio_postgres::NoTls,
    )?;

    println!("Loading from database...");
    let mut db_repo = db::DatabaseRepo::new(&mut client).unwrap();
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

#[cfg(feature = "lite")]
fn try_main() -> Result<(), Error> {
    let mut client = Client::connect(
        "host=localhost user=akroyd_test password=akroyd_test",
        tokio_postgres::NoTls,
    )?;

    println!("Migrating database...");
    db::fast_forward_migrate(&mut client, MIGRATIONS.load()).unwrap();

    println!("Done.");
    Ok(())
}
