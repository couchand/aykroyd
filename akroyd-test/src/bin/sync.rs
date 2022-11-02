use akroyd::*;
use akroyd_test::*;

fn sync_main() {
    use postgres::{Client, NoTls};

    let mut client = &mut Client::connect(
        "host=localhost user=akroyd_test password=akroyd_test",
        NoTls,
    )
    .expect("db conn");

    for customer in client.run(&GetCustomers).expect("query") {
        println!("Got customer: {:?}", customer);
    }

    for customer in client.run(&GetCustomers2).expect("query") {
        println!("Got customer: {:?}", customer);
    }
}

fn main() {
    println!("Testing synchronous client...");
    sync_main();
    println!("Done.");
}
