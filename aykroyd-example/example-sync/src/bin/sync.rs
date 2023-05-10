use aykroyd::sync_client::Client;
use common::*;

fn run_test(client: &mut Client) -> Result<(), postgres::Error> {
    let tim = "Tim";

    println!("Inserting test data...");
    {
        let mut txn = client.transaction()?;

        txn.execute(&InsertCustomer { name: "Red", id: 1 })?;
        txn.execute(&InsertCustomer {
            name: "Herring",
            id: 42,
        })?;

        txn.rollback()?;

        let mut txn = client.transaction()?;

        txn.execute(&InsertCustomer { name: "Jan", id: 1 })?;
        txn.execute(&InsertCustomer { name: tim, id: 42 })?;

        txn.commit()?;
    }

    println!("Querying all customers...");
    for customer in client.query(&GetCustomers)? {
        println!("Got customer: {:?}", customer);
    }

    println!("Querying all customers another way...");
    for customer in client.query(&GetCustomers2)? {
        println!("Got customer: {:?}", customer);
    }

    println!("Querying all customers a third way...");
    for customer in client.query(&GetCustomers3)? {
        println!("Got customer: {:?}", customer);
    }

    println!("Querying all customers a fourth way...");
    for customer in client.query(&GetCustomers4)? {
        println!("Got customer: {:?}", customer);
    }

    println!("Querying all customers a fifth way...");
    for customer in client.query(&GetCustomers5)? {
        println!("Got customer: {:?}", customer);
    }

    println!("Searching for customers with name ending 'm'...");
    for customer in client.query(&SearchCustomersByName("%m"))? {
        println!("Got customer: {:?}", customer);
    }

    println!("Getting customer by id 42...");
    let customer = client.query_one(&GetCustomer::by_id(42))?;
    println!("Got customer: {:?}", customer);

    println!("Getting customer by id 5...");
    let maybe_customer = client.query_opt(&GetCustomer::by_id(5))?;
    println!("Got customer: {:?}", maybe_customer);

    Ok(())
}

fn sync_main() -> bool {
    let mut client = Client::connect(
        "host=localhost user=aykroyd_test password=aykroyd_test",
        postgres::NoTls,
    )
    .expect("db conn");

    println!("Creating table...");
    client
        .as_mut()
        .batch_execute("CREATE TABLE customers (id SERIAL PRIMARY KEY, name TEXT);")
        .expect("setup");

    let ok = match run_test(&mut client) {
        Ok(_) => true,
        Err(e) => {
            eprintln!("Error: {e}");
            false
        }
    };
    println!("Test complete.");

    println!("Dropping table...");
    client
        .as_mut()
        .batch_execute("DROP TABLE customers;")
        .expect("setup");

    ok
}

fn main() {
    println!("Testing synchronous client...");
    let ok = sync_main();
    println!("Done{}.", if ok { "" } else { " (with errors)" });
}
