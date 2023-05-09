use akroyd_migrate::*;

static MIGRATIONS: embedded::EmbeddedRepo = include_migrations!();

fn main() {
    try_main().unwrap()
}

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
    let mut db_repo = db::DatabaseRepo::new(&mut client).unwrap();
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
