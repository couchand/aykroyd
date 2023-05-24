#[cfg(feature = "async")]
use aykroyd::async_client::connect;
#[cfg(feature = "sync")]
use aykroyd::sync_client::Client;
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
    let fs_repo = fs::FsRepo::new("./migrations").expect("No migrations dir found");
    let local_repo = fs_repo.into_local()?;
    println!("Local: {local_repo:?}");

    let mut client = Client::connect(
        "host=localhost user=aykroyd_test password=aykroyd_test",
        tokio_postgres::NoTls,
    )?;

    let db_repo = db::SyncRepo::from_client(&mut client)?;
    println!("DB: {db_repo:?}");

    let plan = plan::Plan::from_db_and_local(&db_repo, &local_repo)?;
    println!("Plan: {plan:?}");

    println!("Applying....");

    db_repo.apply(&plan)?;

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

    let fs_repo = fs::FsRepo::new("./migrations").expect("No migrations dir found");
    let local_repo = fs_repo.into_local()?;
    println!("Local: {local_repo:?}");

    let db_repo = db::AsyncRepo::from_client(&mut client).await?;
    println!("DB: {db_repo:?}");

    let plan = plan::Plan::from_db_and_local(&db_repo, &local_repo)?;
    println!("Plan: {plan:?}");

    println!("Applying....");

    db_repo.apply(&plan).await?;

    println!("Done.");

    Ok(())
}
