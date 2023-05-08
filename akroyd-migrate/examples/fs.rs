use akroyd_migrate::*;

fn main() {
    try_main().unwrap()
}

fn try_main() -> Result<(), fs::CheckError> {
    let mut fs_repo = fs::FsRepo::new("./migrations");

    fs_repo.check()?;

    println!("{fs_repo:?}");

    Ok(())
}
