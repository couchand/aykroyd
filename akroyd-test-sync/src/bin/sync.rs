use akroyd::*;
use akroyd_test::*;

fn sync_main() {
    use postgres::{Client, NoTls};

    let client = &mut Client::connect(
        "host=localhost user=akroyd_test password=akroyd_test",
        NoTls,
    )
    .expect("db conn");

    for customer in GetCustomers.run(client).expect("query") {
        println!("Got customer: {:?}", customer);
    }

    for customer in GetCustomers2.run(client).expect("query") {
        println!("Got customer: {:?}", customer);
    }

    for customer in SearchCustomersByName("%m".into()).run(client).expect("query") {
        println!("Got customer: {:?}", customer);
    }
}

fn main() {
    println!("Testing synchronous client...");
    sync_main();
    println!("Done.");
}
