use aykroyd::async_client::connect;
use aykroyd_migrate::*;

#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand, Debug)]
enum Command {
    /// Get the current database schema migration status
    Status,

    /// Create a new migration
    Create {
        /// The name of the new migration
        migration_name: String,
    },

    /// Update the database to match the local schema
    Apply,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    use clap::Parser;
    let args = Args::parse();

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

    let mut fs_repo = loop {
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

    match &args.command {
        Command::Status | Command::Apply => {
            let local_repo = fs_repo.into_local()?;
            println!("Local: {local_repo:?}");

            let db_repo = db::AsyncRepo::from_client(&mut client).await?;
            println!("DB: {db_repo:?}");

            let plan = plan::Plan::from_db_and_local(&db_repo, &local_repo)?;
            println!("Plan: {plan:?}");

            if matches!(&args.command, Command::Apply) {
                println!("Applying....");

                db_repo.apply(&plan).await?;

                println!("Done.");
            }
        }
        Command::Create { migration_name } => {
            if let Err(e) = fs_repo.add_migration(&migration_name) {
                eprintln!("Error creating migration: {e}");
                std::process::exit(-1);
            }
            println!("Created migration {migration_name}.");
        }
    }

    Ok(())
}
