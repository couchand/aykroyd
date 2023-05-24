use aykroyd::async_client::connect;
use aykroyd_migrate::*;
use aykroyd_migrate::traits::AsyncApply;

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

    /// Update the local repo to match the source migrations
    Commit,

    /// Generate a plan to update the database to the local schema
    Plan,

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

    match &args.command {
        Command::Status => {
            let source_repo = get_source_repo("./migrations");
            let local_repo = source_repo.into_local().unwrap();
            println!("Local: {local_repo:?}");

            let fs_repo = get_fs_repo("./.myg");
            println!("FS: {fs_repo:?}");

            let diff = plan::Diff::from_db_and_local(&fs_repo, &local_repo)?;
            println!("Diff: {diff:?}");
        }
        Command::Commit => {
            let source_repo = get_source_repo("./migrations");
            let local_repo = source_repo.into_local().unwrap();
            println!("Local: {local_repo:?}");

            let fs_repo = get_fs_repo("./.myg");
            println!("FS: {fs_repo:?}");

            let plan = plan::Plan::from_db_and_local(&fs_repo, &local_repo)?;
            println!("Plan: {plan:?}");

            println!("Applying....");

            fs_repo.apply(&plan).await?;

            println!("Done.");
        }
        Command::Plan | Command::Apply => {
            let fs_repo = get_fs_repo("./.myg");
            println!("FS: {fs_repo:?}");

            let db_repo = db::AsyncRepo::from_client(&mut client).await?;
            println!("DB: {db_repo:?}");

            let plan = plan::Plan::from_db_and_local(&db_repo, &fs_repo)?;
            println!("Plan: {plan:?}");

            if matches!(&args.command, Command::Apply) {
                println!("Applying....");

                db_repo.apply(&plan).await?;

                println!("Done.");
            }
        }
        Command::Create { migration_name } => {
            let mut source_repo = get_source_repo("./migrations");
            if let Err(e) = source_repo.add_migration(&migration_name) {
                eprintln!("Error creating migration: {e}");
                std::process::exit(-1);
            }
            println!("Created migration {migration_name}.");
        }
    }

    Ok(())
}

fn get_source_repo<P: AsRef<std::path::Path>>(migrations_dir: P) -> source::SourceRepo {
    let migrations_dir = migrations_dir.as_ref();
    loop {
        match source::SourceRepo::new(migrations_dir) {
            Ok(repo) => break repo,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                std::fs::create_dir(migrations_dir).unwrap();
            }
            Err(err) => {
                eprintln!("Error opening migrations dir: {err}");
                std::process::exit(-1);
            }
        }
    }
}

fn get_fs_repo<P: AsRef<std::path::Path>>(migrations_dir: P) -> fs::FsRepo {
    match fs::FsRepo::new(migrations_dir) {
        Ok(repo) => repo,
        Err(err) => {
            eprintln!("Error opening migrations dir: {err}");
            std::process::exit(-1);
        }
    }
}
