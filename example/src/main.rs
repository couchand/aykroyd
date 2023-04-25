fn main() {
    let migrations = akroyd_migrate::include_migrations!();
    let repo = akroyd_migrate::embedded::EmbeddedRepo::load(&migrations).unwrap();

    println!("{:?}", repo);
}
