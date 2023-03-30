use akroyd::*;

#[derive(Query)]
#[query(text = "SELECT id, name FROM customers", row(Customer))]
pub struct GetCustomers;

#[derive(Debug, FromRow)]
pub struct Customer {
    id: i32,
    name: String,
}

impl Customer {
    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Query)]
#[query(file = "get_customers.sql", row(Customer2))]
pub struct GetCustomers2;

#[derive(Debug, FromRow)]
pub struct Customer2(i32, String);

#[derive(Query)]
#[query(file = "get_customers.sql", row(Customer3))]
pub struct GetCustomers3;

#[derive(Debug, FromRow)]
pub struct Customer3 {
    #[query(column = 0)]
    pub database_id: i32,
    #[query(column = 1)]
    pub customer_name: String,
}

#[derive(Query)]
#[query(file = "get_customers.sql", row(Customer4))]
pub struct GetCustomers4;

#[derive(Debug, FromRow)]
pub struct Customer4(
    #[query(column = "id")] i32,
    #[query(column = "name")] String,
);

#[derive(Statement)]
#[query(text = "SELECT id, name FROM customers", row((i32, String)))]
pub struct GetCustomers5;

impl Query for GetCustomers5 {}

#[derive(Query)]
#[query(
    text = "SELECT name, id FROM customers WHERE name LIKE $1",
    row(Customer)
)]
pub struct SearchCustomersByName<'a>(pub &'a str);

#[derive(QueryOne)]
#[query(text = "SELECT id, name FROM customers WHERE id = $1", row(Customer))]
pub struct GetCustomer {
    id: i32,
}

impl GetCustomer {
    pub fn by_id(id: i32) -> Self {
        GetCustomer { id }
    }
}

#[derive(Statement)]
#[query(text = "INSERT INTO customers (id, name) VALUES ($1, $2)")]
pub struct InsertCustomer<'a> {
    #[query(param = "$2")]
    pub name: &'a str,
    #[query(param = "$1")]
    pub id: i32,
}
