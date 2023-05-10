use aykroyd::async_client::{connect, Client};
use common::*;

async fn run_test(client: &mut Client) -> Result<(), tokio_postgres::Error> {
    client.prepare::<InsertCustomer>().await?;
    let tim = "Tim";

    println!("Inserting test data...");
    {
        let mut txn = client.transaction().await?;

        txn.execute(&InsertCustomer { name: "Red", id: 1 }).await?;
        txn.execute(&InsertCustomer {
            name: "Herring",
            id: 42,
        })
        .await?;

        txn.rollback().await?;

        let mut txn = client.transaction().await?;

        txn.execute(&InsertCustomer { name: "Jan", id: 1 }).await?;
        txn.execute(&InsertCustomer { name: tim, id: 42 }).await?;

        txn.commit().await?;
    }

    println!("Querying all customers...");
    for customer in client.query(&GetCustomers).await? {
        println!("Got customer: {:?}", customer);
    }

    println!("Querying all customers another way...");
    for customer in client.query(&GetCustomers2).await? {
        println!("Got customer: {:?}", customer);
    }

    println!("Querying all customers a third way...");
    for customer in client.query(&GetCustomers3).await? {
        println!("Got customer: {:?}", customer);
    }

    println!("Querying all customers a fourth way...");
    for customer in client.query(&GetCustomers4).await? {
        println!("Got customer: {:?}", customer);
    }

    println!("Querying all customers a fifth way...");
    for customer in client.query(&GetCustomers5).await? {
        println!("Got customer: {:?}", customer);
    }

    println!("Searching for customers with name ending 'm'...");
    for customer in client.query(&SearchCustomersByName("%m")).await? {
        println!("Got customer: {:?}", customer);
    }

    println!("Getting customer by id 42...");
    let customer = client.query_one(&GetCustomer::by_id(42)).await?;
    println!("Got customer: {:?}", customer);

    println!("Getting customer by id 5...");
    let customer = client.query_opt(&GetCustomer::by_id(5)).await?;
    println!("Got customer: {:?}", customer);

    Ok(())
}

async fn async_main() -> bool {
    use tokio_postgres::NoTls;
    let (mut client, worker) = connect(
        "host=localhost user=aykroyd_test password=aykroyd_test",
        NoTls,
    )
    .await
    .expect("db conn");

    tokio::spawn(async move {
        if let Err(e) = worker.await {
            eprintln!("connection error: {}", e);
        }
    });

    println!("Creating table...");
    client
        .as_ref()
        .batch_execute("CREATE TABLE customers (id SERIAL PRIMARY KEY, name TEXT);")
        .await
        .expect("setup");

    let ok = match run_test(&mut client).await {
        Ok(_) => true,
        Err(e) => {
            eprintln!("Error: {e}");
            false
        }
    };
    println!("Test complete.");

    println!("Dropping table...");
    client
        .as_ref()
        .batch_execute("DROP TABLE customers;")
        .await
        .expect("setup");

    ok
}

#[tokio::main]
async fn main() {
    println!("testing asynchronous client...");
    let ok = async_main().await;
    println!("Done{}.", if ok { "" } else { " (with errors)" });
}
