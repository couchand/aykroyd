use akroyd::*;
use akroyd_test::*;

fn sync_main() {
    use postgres::{Client, NoTls};

    let mut client = Client::connect(
        "host=localhost user=akroyd_test password=akroyd_test",
        NoTls,
    )
    .expect("db conn");

    println!("Creating table & inserting data...");
    client.batch_execute("CREATE TABLE customers (id SERIAL PRIMARY KEY, name TEXT); INSERT INTO customers (id, name) VALUES (1, 'Jan'), (2, 'Tim');").expect("setup");

    println!("Querying all customers...");
    for customer in client.run(&GetCustomers).expect("query") {
        println!("Got customer: {:?}", customer);
    }

    println!("Querying all customers another way...");
    for customer in client.run(&GetCustomers2).expect("query") {
        println!("Got customer: {:?}", customer);
    }

    println!("Searching for customers with name ending 'm'...");
    for customer in client.run(&SearchCustomersByName("%m")).expect("query") {
        println!("Got customer: {:?}", customer);
    }

    println!("Getting customer by id 1...");
    let customer = client.run_one(&GetCustomer::by_id(1)).expect("query");
    println!("Got customer: {:?}", customer);

    println!("Dropping table...");
    client.batch_execute("DROP TABLE customers;").expect("setup");
}

fn main() {
    println!("Testing synchronous client...");
    sync_main();
    println!("Done.");
}
