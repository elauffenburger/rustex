use rustex::parser::*;

#[test]
fn test_parse_alphanum() {
    let parser = Parser::new();

    let parsed = parser
        .parse_str("hello world 123")
        .expect("failed to parse");

    insta::assert_debug_snapshot!(parsed);
}

#[test]
fn test_parse_groups() {
    let parser = Parser::new();

    let parsed = parser
        .parse_str("he(llo (1(23) wor)ld i am (?<my_group>named) (?:unnamed) groups)")
        .expect("failed to parse");

    insta::assert_debug_snapshot!(parsed);
}

#[test]
fn test_parse_sets() {
    let parser = Parser::new();

    let parsed = parser
        .parse_str("hel[^lo] (123) w[orld]")
        .expect("failed to parse");

    insta::assert_debug_snapshot!(parsed);
}

#[test]
fn test_parse_escaped() {
    let parser = Parser::new();

    let parsed = parser
        .parse_str("foo\\[ bar\\\\ baz\\^")
        .expect("failed to parse");

    insta::assert_debug_snapshot!(parsed);
}

#[test]
fn test_parse_or() {
    let parser = Parser::new();

    let parsed = parser
        .parse_str("(foo)|((bar)|(baz)qux)")
        .expect("failed to parse");

    insta::assert_debug_snapshot!(parsed);
}

#[test]
fn test_parse_repitition_range() {
    let parser = Parser::new();

    let parsed = parser
        .parse_str("(foo){0,5}bar{1}")
        .expect("failed to parse");

    insta::assert_debug_snapshot!(parsed);
}

#[test]
fn test_parse_modifiers() {
    let parser = Parser::new();

    let parsed = parser
        .parse_str("foo*bar+(baz)?qu?x")
        .expect("failed to parse");

    insta::assert_debug_snapshot!(parsed);
}

#[test]
fn test_unexpected_char_err() {
    let parser = Parser::new();

    let err = parser
        .parse_str("foo\\!bar")
        .expect_err("expected parse failure");

    insta::assert_debug_snapshot!(err);
}
