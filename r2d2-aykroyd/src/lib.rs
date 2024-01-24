//! Aykroyd support for the `r2d2` connection pool.
//!
//! Examples
//! --------
#![cfg_attr(
    feature = "postgres",
    doc = r##"

An example of the `postgres` client.

```no_run
use std::thread;
use postgres::NoTls;
use r2d2_aykroyd::postgres::AykroydConnectionManager;

#[derive(aykroyd::QueryOne)]
#[aykroyd(row(Row), text = "SELECT 1 + $1")]
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
"##)]
#![cfg_attr(
    feature = "mysql",
    doc = r##"

An example of the `mysql` client.

```no_run
use std::thread;
use r2d2_aykroyd::mysql::AykroydConnectionManager;

#[derive(aykroyd::QueryOne)]
#[aykroyd(row(Row), text = "SELECT 1 + ?")]
struct AddOneTo(i32);

#[derive(aykroyd::FromRow)]
struct Row(i32);

fn main() {
    let opts = mysql::Opts::from_url(
        "mysql://user:password@locahost:3307/db_name",
    ).unwrap();
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
"##)]
#![cfg_attr(
    feature = "rusqlite",
    doc = r##"

An example of the `rusqlite` client.

```no_run
use std::thread;
use r2d2_aykroyd::rusqlite::AykroydConnectionManager;

#[derive(aykroyd::QueryOne)]
#[aykroyd(row(Row), text = "SELECT 1 + $1")]
struct AddOneTo(i32);

#[derive(aykroyd::FromRow)]
struct Row(i32);

fn main() {
    let manager = AykroydConnectionManager::file("file.db");
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
"##)]
#![deny(missing_docs, missing_debug_implementations)]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "mysql")]
#[cfg_attr(docsrs, doc(cfg(feature = "mysql")))]
pub mod mysql;

#[cfg(feature = "postgres")]
#[cfg_attr(docsrs, doc(cfg(feature = "postgres")))]
pub mod postgres;

#[cfg(feature = "rusqlite")]
#[cfg_attr(docsrs, doc(cfg(feature = "rusqlite")))]
pub mod rusqlite;
