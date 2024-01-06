use std::fmt::Debug;

use rustex::{
    executor::{self, ExecResult},
    parser,
};

struct FormattableExecResult<'pattern, 'input> {
    result: ExecResult,
    pattern: &'pattern str,
    input: &'input str,
}

impl Debug for FormattableExecResult<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.result.fmt(f)?;
        f.write_str("\n")?;

        f.write_fmt(format_args!("p: '{}'\n", self.pattern))?;
        f.write_fmt(format_args!("i: '{}'\n", self.input))?;

        f.write_fmt(format_args!(
            "    {}^{}^",
            " ".repeat(self.result.start as usize),
            " ".repeat((self.result.end - self.result.start) - 1 as usize),
        ))
    }
}

fn run_test<'p, 'i>(pattern: &'p str, input: &'i str) -> FormattableExecResult<'p, 'i> {
    let parser = parser::Parser::new();
    let mut executor = executor::Executor::new();

    let result = executor
        .exec(parser.parse_str(pattern).expect("should parse"), input)
        .expect("should exec")
        .expect("expected exec result");

    FormattableExecResult {
        result,
        pattern,
        input,
    }
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
    let result = run_test("hel{2}o wo{2,5}rld fo{1,} bar", "hello woorld foooo bar");

    insta::assert_debug_snapshot!(result);
}

#[test]
fn test_repetition() {
    let result = run_test("fo*b* fo+b? ba{1,3}r{2}", "foo fooo baarr");

    insta::assert_debug_snapshot!(result);
}

#[test]
fn test_groups() {
    let result = run_test(
        "(?<one>[^ ]+) (?:world) (?<two>foo) ",
        "hello world foo bar baz",
    );

    insta::assert_debug_snapshot!(result);
}

#[test]
fn test_lazy_match() {
    let result = run_test("(?<one>.*?) (?<two>.*?) (?<three>.+?)", "f bar baz qux");

    insta::assert_debug_snapshot!(result);
}

#[test]
fn test_or() {
    let result = run_test("a (b|c) (c|d)(d|(foo)) (foo|end)", "a b cd end");

    insta::assert_debug_snapshot!(result);
}

// #[test]
// fn test_ps() {
//     let result = run_test(
//         "(?<user>otacon) {4}(?<pid>[0123456789]+) +(?<cpu>[0123456789]\\.[0123456789]) +(?<mem>[0123456789]\\.[0123456789]) +(?<vsz>[0123456789]+) +(?<rss>[0123456789]+) +(?<tty>[^ ]+) +(?<stat>(?:R|W|X)\\+?) {3}(<start>[^ ]+) +(<time>[^ ]+) (<command>.*)", 
//         "otacon    730061  0.0  0.0   7480  3112 pts/32   R+   11:44   0:00 ps aux",
//     );

//     insta::assert_debug_snapshot!(result);
// }
