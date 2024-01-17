use aykroyd::FromRow;

#[derive(FromRow)]
#[aykroyd(by_name)]
struct Test {
    #[aykroyd(column = 1)]
    column_1: i32,
    column_0: i32,
}

fn main() {
}
