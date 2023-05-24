#[cfg(any(feature = "async", feature = "sync"))]
use aykroyd_migrate::*;
#[cfg(feature = "sync")]
use aykroyd_migrate::traits::Apply;
#[cfg(feature = "async")]
use aykroyd_migrate::traits::AsyncApply;

#[cfg(all(not(feature = "sync"), not(feature = "async")))]
fn main() {
}

#[cfg(all(feature = "sync", feature = "async"))]
#[tokio::main]
async fn main() -> Result<(), Error> {
    try_main_sync()?;
    try_main_async().await
}

#[cfg(all(feature = "sync", not(feature = "async")))]
fn main() {
    try_main_sync().unwrap()
}

#[cfg(feature = "sync")]
fn try_main_sync() -> Result<(), Error> {
    let source_repo = source::SourceRepo::new("./migrations").expect("No migrations dir found");
    println!("Source: {source_repo:?}");

    let local_repo = source_repo.into_local()?;
    println!("Local: {local_repo:?}");

    let fs_repo = fs::FsRepo::new("./.myg").expect("Unable to load migrations");
    println!("FS: {fs_repo:?}");

    let plan = plan::Plan::from_db_and_local(&fs_repo, &local_repo)?;
    println!("Plan: {plan:?}");

    println!("Applying....");

    fs_repo.apply(&plan)?;

    println!("Done.");

    Ok(())
}

#[cfg(all(feature = "async", not(feature = "sync")))]
#[tokio::main]
async fn main() -> Result<(), Error> {
    try_main_async().await
}

#[cfg(feature = "async")]
async fn try_main_async() -> Result<(), Error> {
    let source_repo = source::SourceRepo::new("./migrations").expect("No migrations dir found");
    println!("Source: {source_repo:?}");

    let local_repo = source_repo.into_local()?;
    println!("Local: {local_repo:?}");

    let fs_repo = fs::FsRepo::new("./.myg").expect("Unable to load migrations");
    println!("FS: {fs_repo:?}");

    let plan = plan::Plan::from_db_and_local(&fs_repo, &local_repo)?;
    println!("Plan: {plan:?}");

    println!("Applying....");

    fs_repo.apply(&plan).await?;

    println!("Done.");

    Ok(())
}
