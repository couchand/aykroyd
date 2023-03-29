use akroyd_test::*;

fn run_test(client: &mut akroyd::Client) -> Result<(), postgres::Error> {
    println!("Inserting test data...");
    client.execute(&InsertCustomer { name: "Jan" })?;
    client.execute(&InsertCustomer { name: "Tim" })?;

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

    println!("Searching for customers with name ending 'm'...");
    for customer in client.query(&SearchCustomersByName("%m"))? {
        println!("Got customer: {:?}", customer);
    }

    println!("Getting customer by id 1...");
    let customer = client.query_one(&GetCustomer::by_id(1))?;
    println!("Got customer: {:?}", customer);

    println!("Getting customer by id 5...");
    let maybe_customer = client.query_opt(&GetCustomer::by_id(5))?;
    println!("Got customer: {:?}", maybe_customer);

    Ok(())
}

fn sync_main() -> bool {
    let mut client = akroyd::Client::connect(
        "host=localhost user=akroyd_test password=akroyd_test",
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
