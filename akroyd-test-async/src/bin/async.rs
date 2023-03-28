use akroyd::*;
use akroyd_test::*;

async fn run_test(client: &tokio_postgres::Client) -> Result<(), tokio_postgres::Error> {
    println!("Querying all customers...");
    for customer in client.run(&GetCustomers).await? {
        println!("Got customer: {:?}", customer);
    }

    println!("Querying all customers another way...");
    for customer in client.run(&GetCustomers2).await? {
        println!("Got customer: {:?}", customer);
    }

    println!("Querying all customers a third way...");
    for customer in client.run(&GetCustomers3).await? {
        println!("Got customer: {:?}", customer);
    }

    println!("Searching for customers with name ending 'm'...");
    for customer in client.run(&SearchCustomersByName("%m")).await? {
        println!("Got customer: {:?}", customer);
    }

    println!("Getting customer by id 1...");
    let customer = client.run_one(&GetCustomer::by_id(1)).await?;
    println!("Got customer: {:?}", customer);

    Ok(())
}

async fn async_main() -> bool {
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

    println!("Creating table & inserting data...");
    client.batch_execute("CREATE TABLE customers (id SERIAL PRIMARY KEY, name TEXT); INSERT INTO customers (id, name) VALUES (1, 'Jan'), (2, 'Tim');").await.expect("setup");

    let ok = match run_test(&client).await {
        Ok(_) => true,
        Err(e) => {
            eprintln!("Error: {e}");
            false
        }
    };

    println!("Dropping table...");
    client.batch_execute("DROP TABLE customers;").await.expect("setup");

    ok
}

#[tokio::main]
async fn main() {
    println!("testing asynchronous client...");
    let ok = async_main().await;
    println!("Done{}.", if ok { "" } else { " (with errors)" });
}
