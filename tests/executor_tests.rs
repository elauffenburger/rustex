use rustex::{
    executor::{self, ExecResult},
    parser,
};

fn run_test(pattern: &str, input: &str) -> ExecResult {
    let parser = parser::Parser::new();
    let mut executor = executor::Executor::new();

    executor
        .exec(parser.parse_str(pattern).expect("should parse"), input)
        .expect("should exec")
        .expect("expected exec result")
}

#[test]
fn test_start_end() {
    let result = run_test("^foo$", "foo");

    insta::assert_debug_snapshot!(result);
}

#[test]
fn test_partial_word_match() {
    let result = run_test("bar", "foo bar baz");

    insta::assert_debug_snapshot!(result);
}

#[test]
fn test_set() {
    let result = run_test("fo[oa]b[^ob]r", "foobar baz");

    insta::assert_debug_snapshot!(result);
}

#[test]
fn test_repetition() {
    let result = run_test("fo*b* fo+ bar", "foo fooo bar");

    insta::assert_debug_snapshot!(result);
}

