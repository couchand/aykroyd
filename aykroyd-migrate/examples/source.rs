use aykroyd_migrate::*;

fn main() {
    try_main().unwrap()
}

fn try_main() -> Result<(), source::CheckError> {
    let source_repo = source::SourceRepo::new("./migrations").expect("No migrations dir found.");

    let local_repo = source_repo.into_local()?;

    println!("{local_repo:?}");

    Ok(())
}
