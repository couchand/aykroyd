use aykroyd::FromRow;

#[derive(FromRow)]
struct Test {
    #[aykroyd(column = "one")]
    column_1: i32,
    #[aykroyd(column = 0)]
    column_0: i32,
}

fn main() {
}
