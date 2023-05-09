use akroyd_migrate::*;

fn main() {
    try_main().unwrap()
}

fn try_main() -> Result<(), fs::CheckError> {
    let fs_repo = fs::FsRepo::new("./migrations");

    let local_repo = fs_repo.into_local()?;

    println!("{local_repo:?}");

    Ok(())
}
