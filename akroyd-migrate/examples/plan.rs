use akroyd_migrate::*;

fn main() {
    try_main().unwrap()
}

fn try_main() -> Result<(), Error> {
    let fs_repo = fs::FsRepo::new("./migrations");
    let mut local_repo = fs_repo.into_local()?;
    println!("Local: {local_repo:?}");

    let mut db_repo = plan::DbRepo;
    println!("DB: {db_repo:?}");

    let plan = plan::Plan::from_db_and_local(&mut db_repo, &mut local_repo)?;
    println!("Plan: {plan:?}");

    Ok(())
}

#[derive(Debug)]
enum Error {
    Check(fs::CheckError),
    Plan(plan::PlanError),
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
