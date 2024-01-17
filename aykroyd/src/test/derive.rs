#![allow(dead_code)]

use aykroyd::row::FromColumnsIndexed;
use aykroyd::{FromRow, Statement};

use super::sync_client::TestClient;

#[test]
fn compile_fail() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/from-row/*.rs");
}

fn assert_num_columns<T: FromColumnsIndexed<TestClient>>(expected: usize) {
    assert_eq!(expected, <T as FromColumnsIndexed<TestClient>>::NUM_COLUMNS);
}

#[test]
fn explicit_column_count_basic() {
    #[derive(FromRow)]
    struct Row {
        #[aykroyd(column = 5)]
        column_5: i32,
    }

    assert_num_columns::<Row>(6);
}

#[test]
fn explicit_column_count_nested() {
    #[derive(FromColumnsIndexed)]
    struct Nested {
        one: i32,
        two: i32,
        three: i32,
    }

    #[derive(FromRow)]
    struct Row {
        #[aykroyd(nested, column = 3)]
        nested: Nested,
    }

    assert_num_columns::<Row>(6);
}

#[test]
fn implicit_column_count_basic() {
    #[derive(FromRow)]
    #[aykroyd(by_index)]
    struct Row {
        one: i32,
        two: i32,
        three: i32,
        four: i32,
        five: i32,
        six: i32,
    }

    assert_num_columns::<Row>(6);
}

#[test]
fn implicit_column_count_nested() {
    #[derive(FromColumnsIndexed)]
    struct Nested {
        four: i32,
        five: i32,
        six: i32,
    }

    #[derive(FromRow)]
    #[aykroyd(by_index)]
    struct Row {
        one: i32,
        two: i32,
        three: i32,
        #[aykroyd(nested)]
        nested: Nested,
    }

    assert_num_columns::<Row>(6);
}

#[test]
fn statement_explicit_param() {
    #[derive(Statement)]
    #[aykroyd(text = "")]
    struct MyStatement<'a> {
        #[aykroyd(param = "$2")]
        second: &'a str,
        first: &'a str,
    }

    let mut client = TestClient::new();

    client.execute(&MyStatement {
        first: "first",
        second: "second",
    }).unwrap();

    let records = client.records();
    assert_eq!(1, records.len());
    assert!(records[0].params.is_some());

    let params = records[0].params.as_ref().unwrap();
    assert_eq!(2, params.len());
    assert_eq!("first", params[0]);
    assert_eq!("second", params[1]);
}
