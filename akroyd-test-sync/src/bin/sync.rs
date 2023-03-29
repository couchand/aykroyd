use akroyd::*;
use akroyd_test::*;

fn run_test(client: &mut postgres::Client) -> Result<(), postgres::Error> {
    println!("Inserting test data...");
    client.exec(&InsertCustomer { name: "Jan" })?;
    client.exec(&InsertCustomer { name: "Tim" })?;

    println!("Querying all customers...");
    for customer in client.run(&GetCustomers)? {
        println!("Got customer: {:?}", customer);
    }

    println!("Querying all customers another way...");
    for customer in client.run(&GetCustomers2)? {
        println!("Got customer: {:?}", customer);
    }

    println!("Querying all customers a third way...");
    for customer in client.run(&GetCustomers3)? {
        println!("Got customer: {:?}", customer);
    }

    println!("Searching for customers with name ending 'm'...");
    for customer in client.run(&SearchCustomersByName("%m"))? {
        println!("Got customer: {:?}", customer);
    }

    println!("Getting customer by id 1...");
    let customer = client.run_one(&GetCustomer::by_id(1))?;
    println!("Got customer: {:?}", customer);

    println!("Getting customer by id 5...");
    let maybe_customer = client.run_opt(&GetCustomer::by_id(5))?;
    println!("Got customer: {:?}", maybe_customer);

    Ok(())
}

fn sync_main() -> bool {
    use postgres::{Client, NoTls};

    let mut client = Client::connect(
        "host=localhost user=akroyd_test password=akroyd_test",
        NoTls,
    )
    .expect("db conn");

    println!("Creating table...");
    client.batch_execute("CREATE TABLE customers (id SERIAL PRIMARY KEY, name TEXT);").expect("setup");

    let ok = match run_test(&mut client) {
        Ok(_) => true,
        Err(e) => {
            eprintln!("Error: {e}");
            false
        }
    };
    println!("Test complete.");

    println!("Dropping table...");
    client.batch_execute("DROP TABLE customers;").expect("setup");

    ok
}

fn main() {
    println!("Testing synchronous client...");
    let ok = sync_main();
    println!("Done{}.", if ok { "" } else { " (with errors)" });
}
