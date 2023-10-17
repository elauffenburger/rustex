use rustex::{executor, parser};

#[test]
fn test_start_end() {
    let parser = parser::Parser::new();
    let mut executor = executor::Executor::new();

    let result = executor
        .exec(parser.parse_str("^foo$").expect("should parse"), "foo")
        .expect("should exec")
        .expect("expected exec result");
}
