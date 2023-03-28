use akroyd::*;

#[derive(Query, ToRow)]
#[query(text = "SELECT id, name FROM customers", results(Customer))]
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

#[derive(Query, ToRow)]
#[query(file = "get_customers.sql", results(Customer2))]
pub struct GetCustomers2;

#[derive(Debug, FromRow)]
pub struct Customer2(i32, String);

#[derive(Query, ToRow)]
#[query(text = "SELECT id, name FROM customers WHERE name LIKE $1", results(Customer))]
pub struct SearchCustomersByName<'a>(pub &'a str);

#[derive(QueryOne, ToRow)]
#[query(text = "SELECT id, name FROM customers WHERE id = $1", result(Customer))]
pub struct GetCustomer {
    id: i32,
}

impl GetCustomer {
    pub fn by_id(id: i32) -> Self {
        GetCustomer { id }
    }
}
