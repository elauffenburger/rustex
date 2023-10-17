use rustex::{executor, parser};

#[test]
fn test_start_end() {
    let parser = parser::Parser::new();
    let mut executor = executor::Executor::new();

    let result = executor
        .exec(parser.parse_str("^foo$").expect("should parse"), "foo")
        .expect("should exec")
        .expect("expected exec result");

    insta::assert_debug_snapshot!(result);
}

#[test]
fn test_partial_word_match() {
    let parser = parser::Parser::new();
    let mut executor = executor::Executor::new();

    let result = executor
        .exec(parser.parse_str("bar").expect("should parse"), "foo bar baz")
        .expect("should exec")
        .expect("expected exec result");

    insta::assert_debug_snapshot!(result);
}