deadpool-aykroyd
================

[Documentation](https://docs.rs/deadpool-aykroyd)

[aykroyd](https://crates.io/crates/aykroyd) support library for the [deadpool](https://crates.io/crates/deadpool) connection pool.

Example
-------

```rust
use deadpool_aykroyd::{Manager, ManagerConfig, Pool, RecyclingMethod};
use tokio_postgres::NoTls;

#[derive(aykroyd::QueryOne)]
#[query(row(Row), text = "SELECT 1 + $1")]
struct AddOneTo(i32);

#[derive(aykroyd::FromRow)]
struct Row(i32);

#[tokio::main]
async fn main() {
    let pg_config = "host=localhost user=postgres".parse().unwrap();
    let mgr_config = ManagerConfig {
        recycling_method: RecyclingMethod::Fast
    };
    let mgr = Manager::from_config(pg_config, NoTls, mgr_config);
    let pool = Pool::builder(mgr).max_size(16).build().unwrap();
    for i in 1..10 {
        let mut client = pool.get().await.unwrap();
        let stmt = client.prepare_cached("SELECT 1 + $1").await.unwrap();
        let row = client.query_onw(&AddOneTo(i)).await.unwrap();
        let value = row.0;
        assert_eq!(value, i + 1);
    }
}
```
