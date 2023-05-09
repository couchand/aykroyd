use akroyd::sync_client::Client;
use akroyd_migrate::*;

fn main() {
    try_main().unwrap()
}

fn try_main() -> Result<(), Error> {
    let fs_repo = fs::FsRepo::new("./migrations");
    let mut local_repo = fs_repo.into_local()?;
    println!("Local: {local_repo:?}");

    let mut client = Client::connect(
        "host=localhost user=akroyd_test password=akroyd_test",
        tokio_postgres::NoTls,
    )?;

    let mut db_repo = db2::DatabaseRepo::new(&mut client)?;
    println!("DB: {db_repo:?}");

    let plan = plan::Plan::from_db_and_local(&mut db_repo, &mut local_repo)?;
    println!("Plan: {plan:?}");

    println!("Applying....");

    db_repo.apply(&plan).unwrap();

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
