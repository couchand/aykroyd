use akroyd::*;
use akroyd_test::*;

async fn async_main() {
    use tokio_postgres::NoTls;
    let (client, worker) = tokio_postgres::connect(
        "host=localhost user=akroyd_test password=akroyd_test",
        NoTls,
    )
    .await
    .expect("db conn");

    tokio::spawn(async move {
        if let Err(e) = worker.await {
            eprintln!("connection error: {}", e);
        }
    });

    for customer in client.run(&GetCustomers).await.expect("query") {
        println!("Got customer: {:?}", customer);
    }

    for customer in client.run(&GetCustomers2).await.expect("query") {
        println!("Got customer: {:?}", customer);
    }

    for customer in client.run(&SearchCustomersByName("%m".into())).await.expect("query") {
        println!("Got customer: {:?}", customer);
    }
}

#[tokio::main]
async fn main() {
    println!("testing asynchronous client...");
    async_main().await;
    println!("Done.");
}
