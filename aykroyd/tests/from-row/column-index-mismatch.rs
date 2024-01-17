use aykroyd::FromRow;

#[derive(FromRow)]
#[aykroyd(by_index)]
struct Test {
    #[aykroyd(column = "one")]
    column_1: i32,
    column_0: i32,
}

fn main() {
}
