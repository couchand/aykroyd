use akroyd_migrate::*;

fn main() {
    let repo = local::LocalRepo::load("./migrations").unwrap();
    println!("{repo:?}");
}
