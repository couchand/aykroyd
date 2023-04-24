use akroyd_migrate::*;

#[cfg(feature = "sync")]
fn main() {
    try_main().unwrap()
}

#[cfg(feature = "sync")]
fn try_main() -> Result<(), tokio_postgres::Error> {
    use akroyd::sync_client::Client;

    let mut local_repo = local::LocalRepo::load("./migrations").unwrap();
    println!("{local_repo:?}");

    let mut client = Client::connect(
        "host=localhost user=akroyd_test password=akroyd_test",
        tokio_postgres::NoTls,
    )?;

    println!("Loading from database...");
    let database_repo = db::DatabaseRepo::from_sync_client(&mut client).unwrap();
    println!("{database_repo:?}");

    println!("Done.");
    Ok(())
}

#[cfg(feature = "async")]
#[tokio::main]
async fn main() -> Result<(), tokio_postgres::Error> {
    use akroyd::async_client::connect;

    let mut local_repo = local::LocalRepo::load("./migrations").unwrap();
    println!("{local_repo:?}");

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

    println!("Done.");
    Ok(())
}
