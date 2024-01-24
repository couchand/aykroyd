bb8-aykroyd
===========

[Documentation](https://docs.rs/bb8-aykroyd)

[aykroyd](https://crates.io/crates/aykroyd) support library for the [bb8](https://crates.io/crates/bb8) connection pool.

Example
-------

```rust
use bb8_aykroyd::AykroydConnectionManager;
use tokio_postgres::tls::NoTls;

#[derive(aykroyd::QueryOne)]
#[query(row(Row), text = "SELECT 1 + $1")]
struct AddOneTo(i32);

#[derive(aykroyd::FromRow)]
struct Row(i32);

#[tokio::main]
async fn main() {
    let manager = AykroydConnectionManager::new(
        "host=localhost user=postgres".parse().unwrap(),
        NoTls,
    );
    let pool = bb8::Pool::builder()
        .max_size(15)
        .build(manager)
        .await
        .unwrap();

    for i in 0..20 {
        let pool = pool.clone();
        tokio::spawn(async move {
            let client = pool.get().await.unwrap();

            let row = client.query_one(&AddOneTo(i)).await.unwrap();
            let value = row.0;
            assert_eq!(value, i + 1);
        });
    }
}
```
