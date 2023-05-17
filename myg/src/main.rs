use aykroyd::async_client::connect;
use aykroyd_migrate::*;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let args: Vec<_> = std::env::args().collect();
    let command: String = args.get(1).cloned().unwrap_or_default();

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

    let fs_repo = loop {
        let migrations_dir = "./migrations";
        match fs::FsRepo::new(migrations_dir) {
            Ok(fs_repo) => break fs_repo,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                std::fs::create_dir(migrations_dir).unwrap();
            }
            Err(err) => {
                eprintln!("Error opening migrations dir: {err}");
                std::process::exit(-1);
            }
        }
    };

    let local_repo = fs_repo.into_local()?;
    println!("Local: {local_repo:?}");

    let db_repo = db::AsyncRepo::from_client(&mut client).await?;
    println!("DB: {db_repo:?}");

    let plan = plan::Plan::from_db_and_local(&db_repo, &local_repo)?;
    println!("Plan: {plan:?}");

    if command == "apply" {
        println!("Applying....");

        db_repo.apply(&plan).await?;

        println!("Done.");
    }

    Ok(())
}
