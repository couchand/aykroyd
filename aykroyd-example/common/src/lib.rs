use aykroyd::*;

#[derive(Query)]
#[aykroyd(text = "SELECT id, name FROM customers", row(Customer))]
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
#[aykroyd(file = "get_customers.sql", row(Customer2))]
pub struct GetCustomers2;

#[derive(Debug, FromRow)]
pub struct Customer2(i32, String);

#[derive(Query)]
#[aykroyd(file = "get_customers.sql", row(Customer3))]
pub struct GetCustomers3;

#[derive(Debug, FromRow)]
pub struct Customer3 {
    #[aykroyd(column = 0)]
    pub database_id: i32,
    #[aykroyd(column = 1)]
    pub customer_name: String,
}

#[derive(Query)]
#[aykroyd(file = "get_customers.sql", row(Customer4))]
pub struct GetCustomers4;

#[derive(Debug, FromRow)]
pub struct Customer4(
    #[aykroyd(column = "id")] i32,
    #[aykroyd(column = "name")] String,
);

#[derive(Query)]
#[aykroyd(text = "SELECT id, name FROM customers", row((i32, String)))]
pub struct GetCustomers5;

#[derive(Query)]
#[aykroyd(
    text = "SELECT name, id FROM customers WHERE name LIKE $1",
    row(Customer)
)]
pub struct SearchCustomersByName<'a>(pub &'a str);

#[derive(QueryOne)]
#[aykroyd(text = "SELECT id, name FROM customers WHERE id = $1", row(Customer))]
pub struct GetCustomer {
    id: i32,
}

impl GetCustomer {
    pub fn by_id(id: i32) -> Self {
        GetCustomer { id }
    }
}

#[derive(Statement)]
#[aykroyd(text = "INSERT INTO customers (id, name) VALUES ($1, $2)")]
pub struct InsertCustomer<'a> {
    #[aykroyd(param = "$2")]
    pub name: &'a str,
    pub id: i32,
}
