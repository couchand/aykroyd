use akroyd::*;
use akroyd_test::*;

fn run_test(client: &mut postgres::Client) -> Result<(), postgres::Error> {
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

    Ok(())
}

fn sync_main() -> bool {
    use postgres::{Client, NoTls};

    let mut client = Client::connect(
        "host=localhost user=akroyd_test password=akroyd_test",
        NoTls,
    )
    .expect("db conn");

    println!("Creating table & inserting data...");
    client.batch_execute("CREATE TABLE customers (id SERIAL PRIMARY KEY, name TEXT); INSERT INTO customers (id, name) VALUES (1, 'Jan'), (2, 'Tim');").expect("setup");

    let ok = match run_test(&mut client) {
        Ok(_) => true,
        Err(e) => {
            eprintln!("Error: {e}");
            false
        }
    };

    println!("Dropping table...");
    client.batch_execute("DROP TABLE customers;").expect("setup");

    ok
}

fn main() {
    println!("Testing synchronous client...");
    let ok = sync_main();
    println!("Done{}.", if ok { "" } else { " (with errors)" });
}
