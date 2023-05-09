use akroyd_migrate::*;

static MIGRATIONS: embedded2::EmbeddedRepo = include_migrations!();

#[cfg(feature = "sync")]
fn main() {
    try_main().unwrap()
}

#[cfg(feature = "sync")]
fn try_main() -> Result<(), Error> {
    use akroyd::sync_client::Client;

    println!("Loading embedded migrations...");
    let mut local_repo = MIGRATIONS.load();
    println!("{local_repo:?}");

    let mut client = Client::connect(
        "host=localhost user=akroyd_test password=akroyd_test",
        tokio_postgres::NoTls,
    )?;

    println!("Loading from database...");
    let mut db_repo = db2::DatabaseRepo::new(&mut client).unwrap();
    println!("{db_repo:?}");

    let plan = plan::Plan::from_db_and_local(&mut db_repo, &mut local_repo)?;
    println!("Plan: {plan:?}");

    if !plan.is_empty() {
        println!("Applying....");

        db_repo.apply(&plan).unwrap();
    }

    println!("Done.");
    Ok(())
}

#[cfg(feature = "async")]
#[tokio::main]
async fn main() -> Result<(), tokio_postgres::Error> {
    use akroyd::async_client::connect;

    println!("Loading embedded migrations...");
    let embedded_repo = MIGRATIONS.load();
    println!("{embedded_repo:?}");

    let (mut client, conn) = connect(
        "host=localhost user=akroyd_test password=akroyd_test",
        tokio_postgres::NoTls,
    ).await?;

    tokio::spawn(async move {
        if let Err(e) = conn.await {
            eprintln!("connection error: {e}");
        }
    });

    println!("Loading from database...");
    let database_repo = db::DatabaseRepo::from_async_client(&mut client).await.unwrap();
    println!("{database_repo:?}");

/*
    let plan = plan::Plan::from_db_and_local(&mut db_repo, &mut local_repo)?;
    println!("Plan: {plan:?}");

    println!("Applying....");

    db_repo.apply(&plan).unwrap();
*/

    println!("Done.");
    Ok(())
}

#[derive(Debug)]
enum Error {
    Check(fs::CheckError),
    Plan(plan::PlanError),
    Db(tokio_postgres::Error),
}

impl From<fs::CheckError> for Error {
    fn from(err: fs::CheckError) -> Self {
        Error::Check(err)
    }
}

impl From<plan::PlanError> for Error {
    fn from(err: plan::PlanError) -> Self {
        Error::Plan(err)
    }
}

impl From<tokio_postgres::Error> for Error {
    fn from(err: tokio_postgres::Error) -> Self {
        Error::Db(err)
    }
}
