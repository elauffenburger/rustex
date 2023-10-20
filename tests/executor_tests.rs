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
fn test_repetition_range() {
    let result = run_test("hel{2}o wo{2,5}rld", "hello woorld");

    insta::assert_debug_snapshot!(result);
}

#[test]
fn test_repetition() {
    let result = run_test("fo*b* fo+b? ba{1,3}r{2}", "foo fooo baarr");

    insta::assert_debug_snapshot!(result);
}

#[test]
fn test_groups() {
    let result = run_test("(?<one>[^ ]+) (?:world) (?<two>foo)", "hello world foo bar");

    insta::assert_debug_snapshot!(result);
}

// #[test]
// fn test_ps() {
//     let result = run_test("(?<user>otacon) {4}(?<pid>[0-9]+) +(?<cpu>[0-9]\\.[0-9]) +(?<mem>[0-9]\\.[0-9]) +(?<vsz>[0-9]+) +(?<rss>[0-9]+) +(?<tty>[^ ]+) +(?<stat>(?:R|W|X)\\+?) {3}(<start>[^ ]+) +(<time>[^ ]+) (<command>.*)", "otacon    730061  0.0  0.0   7480  3112 pts/32   R+   11:44   0:00 ps aux");

//     insta::assert_debug_snapshot!(result);
// }
