use std::{cell::RefCell, rc::Rc};

use rustex::parser::*;

fn rcref<T>(val: T) -> Rc<RefCell<T>> {
    Rc::new(RefCell::new(val))
}

fn node_from_literal_str(val: &str) -> Option<Rc<RefCell<Node>>> {
    Some(rcref(Node {
        val: NodeVal::Word(val.to_string()),
        next: None,
    }))
}

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
