use std::{cell::RefCell, error::Error, mem, rc::Rc};

fn rcref<T>(val: T) -> Rc<RefCell<T>> {
    Rc::new(RefCell::new(val))
}

#[derive(Debug)]
pub struct UnexpectedCharErr {
    ch: char,
}

pub struct Parser {}

impl Parser {
    pub fn new() -> Self {
        Parser {}
    }

    fn parse_iter_until<I>(
        self: &Self,
        iter: &mut std::iter::Peekable<I>,
        until: Option<char>,
    ) -> Result<Option<Rc<RefCell<Node>>>, String>
    where
        I: Iterator<Item = char>,
    {
        let mut head = None;
        let mut node: Option<Rc<RefCell<Node>>> = None;

        while let Some(ch) = iter.next() {
            if let Some(until) = until {
                if until == ch {
                    break;
                }
            }

            let new_node = match ch {
                '(' => {
                    let group = self
                        .parse_iter_until(iter, Some(')'))?
                        .expect("expected group contents");

                    rcref(Node {
                        val: NodeVal::Group(group),
                        next: None,
                    })
                }
                '|' => todo!(),
                '{' => todo!(),
                '[' => {
                    let mut is_inverted = false;
                    if let Some(next) = iter.peek() {
                        if *next == '^' {
                            is_inverted = true;
                            _ = iter.next();
                        }
                    }

                    let mut found_end = false;
                    let mut set = vec![];
                    while let Some(ch) = iter.next() {
                        if ch == ']' {
                            found_end = true;
                            break;
                        }

                        set.push(ch);
                    }

                    if !found_end {
                        return Err(String::from("failed to find end to character set"));
                    }

                    rcref(Node {
                        val: NodeVal::Set(set),
                        next: None,
                    })
                }
                '\\' => {
                    let ch = match iter.next().expect("expected character to quote") {
                        '(' | ')' | '{' | '}' | '[' | ']' | '|' | '\\' | '^' | '$' | '.' | '*'
                        | '?' => ch,
                        _ => return Err(format!("unexpected char {}", ch)),
                    };

                    rcref(Node {
                        val: NodeVal::Char(ch),
                        next: None,
                    })
                }
                '.' => rcref(Node {
                    val: NodeVal::Any,
                    next: None,
                }),
                '*' => rcref(Node {
                    val: NodeVal::ZeroOrMore,
                    next: None,
                }),
                '+' => rcref(Node {
                    val: NodeVal::OneOrMore,
                    next: None,
                }),
                '?' => rcref(Node {
                    val: NodeVal::Optional,
                    next: None,
                }),
                '^' => rcref(Node {
                    val: NodeVal::Start,
                    next: None,
                }),
                '$' => rcref(Node {
                    val: NodeVal::End,
                    next: None,
                }),
                _ => rcref(Node {
                    val: NodeVal::Char(ch),
                    next: None,
                }),
            };

            if node.is_none() {
                head = Some(new_node.clone());
                node = Some(new_node.clone());
            } else {
                // Update node.next to point to the new node.
                let node_val = mem::take(&mut node).unwrap();
                (*node_val).borrow_mut().next = Some(new_node.clone());

                // Update node to point to the new node.
                node = Some(new_node.clone());
            }
        }

        Ok(head)
    }

    pub fn parse(self: &Self, input: &str) -> Result<ParseResult, Box<dyn Error>> {
        let mut iter = input.chars().peekable();

        Ok(ParseResult {
            head: self.parse_iter_until(&mut iter, None)?,
        })
    }
}

#[derive(Default, Debug)]
pub struct ParseResult {
    pub head: Option<Rc<RefCell<Node>>>,
}

#[derive(Debug)]
pub struct Node {
    pub val: NodeVal,
    pub next: Option<Rc<RefCell<Node>>>,
}

#[derive(Debug)]
pub enum NodeVal {
    Char(char),
    Any,
    ZeroOrMore,
    OneOrMore,
    Start,
    End,
    Optional,
    Group(Rc<RefCell<Node>>),
    Set(Vec<char>),
}

#[cfg(test)]
mod test {
    use super::*;

    fn node_from_literal_str(val: &str) -> Option<Rc<RefCell<Node>>> {
        let mut res = None;
        let mut node = None;

        for c in val.chars() {
            let new_node = rcref(Node {
                val: NodeVal::Char(c),
                next: None,
            });

            if node.is_none() {
                node = Some(new_node.clone());
                res = Some(new_node.clone());
            } else {
                let node_val = mem::take(&mut node);
                (*node_val.unwrap()).borrow_mut().next = Some(new_node.clone());

                node = Some(new_node);
            }
        }

        res
    }

    #[test]
    fn test_parse_alphanum() {
        let parser = Parser::new();

        let parsed = parser.parse("hello world 123").expect("failed to parse");
        assert_eq!(
            format!("{:?}", parsed),
            format!(
                "{:?}",
                ParseResult {
                    head: node_from_literal_str("hello world 123")
                }
            ),
        )
    }

    #[test]
    fn test_parse_group() {
        let parser = Parser::new();

        let parsed = parser.parse("hello (123) world").expect("failed to parse");
        println!("{:?}", parsed);
        todo!();
    }

    #[test]
    fn test_parse_set() {
        let parser = Parser::new();

        let parsed = parser.parse("hello (123) w[orld]").expect("failed to parse");
        println!("{:?}", parsed);
        todo!();
    }
}
