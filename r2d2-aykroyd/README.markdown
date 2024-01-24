r2d2-aykroyd
============

[Documentation](https://docs.rs/r2d2-aykroyd)

[aykroyd](https://crates.io/crates/aykroyd) support library for the [r2d2](https://crates.io/crates/r2d2) connection pool.

Examples
--------

An example of the `postgres` client.

```rust
use std::thread;
use postgres::NoTls;
use r2d2_aykroyd::postgres::AykroydConnectionManager;

#[derive(aykroyd::QueryOne)]
#[query(row(Row), text = "SELECT 1 + $1")]
struct AddOneTo(i32);

#[derive(aykroyd::FromRow)]
struct Row(i32);

fn main() {
    let manager = AykroydConnectionManager::new(
        "host=localhost user=postgres".parse().unwrap(),
        NoTls,
    );
    let pool = r2d2::Pool::new(manager).unwrap();

    for i in 0..10i32 {
        let pool = pool.clone();
        thread::spawn(move || {
            let mut client = pool.get().unwrap();
            let row = client.query_one(&AddOneTo(i)).unwrap();
            let value = row.0;
            assert_eq!(value, i + 1);
        });
    }
}
```

An example of the `mysql` client.

```rust
use std::thread;
use r2d2_aykroyd::mysql::AykroydConnectionManager;

#[derive(aykroyd::QueryOne)]
#[query(row(Row), text = "SELECT 1 + $1")]
struct AddOneTo(i32);

#[derive(aykroyd::FromRow)]
struct Row(i32);

fn main() {
    let opts = mysqsl::Opts::from_url(
        "mysql://user:password@locahost:3307/db_name",
    );
    let builder = mysql::OptsBuilder::from_opts(opts);
    let manager = AykroydConnectionManager::new(builder);
    let pool = r2d2::Pool::new(manager).unwrap();

    for i in 0..10i32 {
        let pool = pool.clone();
        thread::spawn(move || {
            let mut client = pool.get().unwrap();
            let row = client.query_one(&AddOneTo(i)).unwrap();
            let value = row.0;
            assert_eq!(value, i + 1);
        });
    }
}
```
