#[cfg(feature = "async")]
use aykroyd::async_client::connect;
#[cfg(feature = "sync")]
use aykroyd::sync_client::Client;
use aykroyd_migrate::*;

static MIGRATIONS: embedded::EmbeddedRepo = include_migrations!();

#[cfg(feature = "sync")]
fn main() {
    try_main().unwrap()
}

#[cfg(all(feature = "full", feature = "sync"))]
fn try_main() -> Result<(), Error> {
    println!("Loading embedded migrations...");
    let local_repo = MIGRATIONS.load();
    println!("{local_repo:?}");

    let mut client = Client::connect(
        "host=localhost user=aykroyd_test password=aykroyd_test",
        tokio_postgres::NoTls,
    )?;

    println!("Loading from database...");
    let db_repo = db::DbRepo::from_client(&mut client)?;
    println!("{db_repo:?}");

    let plan = plan::Plan::from_db_and_local(&db_repo, &local_repo)?;
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

        db_repo.apply(&plan)?;

        println!("Done.");
    } else {
        println!("Nothing to do.");
    }

    Ok(())
}

#[cfg(all(feature = "lite", feature = "sync"))]
fn try_main() -> Result<(), Error> {
    let mut client = Client::connect(
        "host=localhost user=aykroyd_test password=aykroyd_test",
        tokio_postgres::NoTls,
    )?;

    println!("Migrating database...");
    match db::DbRepo::fast_forward_migrate(&mut client, MIGRATIONS.load())? {
        db::MergeStatus::NothingToDo => println!("Nothing to do."),
        db::MergeStatus::Done => println!("Done."),
    }

    Ok(())
}

#[cfg(all(feature = "lite", feature = "async"))]
#[tokio::main]
async fn main() -> Result<(), Error> {
    let (mut client, connection) = connect(
        "host=localhost user=aykroyd_test password=aykroyd_test",
        tokio_postgres::NoTls,
    )
    .await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    println!("Migrating database...");
    match db::DbRepo::fast_forward_migrate(&mut client, MIGRATIONS.load()).await? {
        db::MergeStatus::NothingToDo => println!("Nothing to do."),
        db::MergeStatus::Done => println!("Done."),
    }

    Ok(())
}

#[cfg(all(feature = "full", feature = "async"))]
#[tokio::main]
async fn main() -> Result<(), Error> {
    let (mut client, connection) = connect(
        "host=localhost user=aykroyd_test password=aykroyd_test",
        tokio_postgres::NoTls,
    )
    .await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    println!("Loading embedded migrations...");
    let local_repo = MIGRATIONS.load();
    println!("{local_repo:?}");

    println!("Loading from database...");
    let db_repo = db::DbRepo::from_client(&mut client).await?;
    println!("{db_repo:?}");

    let plan = plan::Plan::from_db_and_local(&db_repo, &local_repo)?;
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

        db_repo.apply(&plan).await?;

        println!("Done.");
    } else {
        println!("Nothing to do.");
    }

    Ok(())
}
