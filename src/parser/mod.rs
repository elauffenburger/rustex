use core::fmt;
use std::{cell::RefCell, error::Error, iter::Peekable, mem, rc::Rc};

fn rcref<T>(val: T) -> Rc<RefCell<T>> {
    Rc::new(RefCell::new(val))
}

#[derive(Debug)]
pub enum ParseError {
    UnexpectedCharErr(char),
    UnterminatedCharSet,
    EmptyCaptureGroup,
    MissingCharacterToEscape,
    MissingRightSideOfOr,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::UnexpectedCharErr(ch) => {
                f.write_fmt(format_args!("unexpected char {}", ch))
            }
            ParseError::UnterminatedCharSet => f.write_str("unterminated character set"),
            ParseError::EmptyCaptureGroup => f.write_str("empty capture group"),
            ParseError::MissingCharacterToEscape => f.write_str("missing character to escape"),
            ParseError::MissingRightSideOfOr => f.write_str("missing right side of or"),
        }
    }
}

pub struct Parser {}

enum GroupType {
    Group,
    Set,
}

struct ParserImpl<Iter>
where
    Iter: Iterator<Item = char>,
{
    iter: Peekable<Iter>,
}

impl<Iter> ParserImpl<Iter>
where
    Iter: Iterator<Item = char>,
{
    fn parse(
        self: &mut Self,
        group_type: Option<GroupType>,
    ) -> Result<Option<Rc<RefCell<Node>>>, ParseError> {
        let mut head = None;
        let mut prev: Option<Rc<RefCell<Node>>> = None;

        while let Some(ch) = self.next() {
            match group_type {
                None => {}
                Some(GroupType::Group) => {
                    if ch == ')' {
                        break;
                    }
                }
                Some(GroupType::Set) => {
                    if ch == ']' {
                        break;
                    }
                }
            }

            let new_node = match ch {
                '{' => todo!(),
                '(' => {
                    let group = match self.parse(Some(GroupType::Group))? {
                        Some(group) => group,
                        None => return Err(ParseError::EmptyCaptureGroup),
                    };

                    rcref(Node {
                        val: NodeVal::Group(group),
                        next: None,
                    })
                }
                '|' => {
                    // Grab the last node val that we created by swapping it out with a placeholder val.
                    let mut left = NodeVal::Any;
                    mem::swap(&mut (prev.as_ref().unwrap()).borrow_mut().val, &mut left);

                    rcref(Node {
                        val: NodeVal::Or {
                            left: rcref(Node {
                                val: left,
                                next: None,
                            }),
                            right: match self.parse(None)? {
                                None => return Err(ParseError::MissingRightSideOfOr),
                                Some(val) => val,
                            },
                        },
                        next: None,
                    })
                }
                '[' => {
                    let mut inverted = false;
                    if let Some(next) = self.peek() {
                        if *next == '^' {
                            inverted = true;
                            _ = self.next();
                        }
                    }

                    let mut found_end = false;
                    let mut set = vec![];
                    while let (Some(ch), escaped) = self.next_escaped()? {
                        if !escaped && ch == ']' {
                            found_end = true;
                            break;
                        }

                        set.push(ch);
                    }

                    if !found_end {
                        return Err(ParseError::UnterminatedCharSet);
                    }

                    rcref(Node {
                        val: NodeVal::Set { set, inverted },
                        next: None,
                    })
                }
                '\\' => rcref(Node {
                    val: NodeVal::Char(self.escape_next()?),
                    next: None,
                }),
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

            if prev.is_none() {
                head = Some(new_node.clone());
                prev = Some(new_node.clone());
            } else {
                // Update node.next to point to the new node.
                let node_val = mem::take(&mut prev).unwrap();
                (*node_val).borrow_mut().next = Some(new_node.clone());

                // Update node to point to the new node.
                prev = Some(new_node.clone());
            }
        }

        Ok(head)
    }

    fn peek(&mut self) -> Option<&char> {
        self.iter.peek()
    }

    fn next(&mut self) -> Option<char> {
        self.iter.next()
    }

    fn next_escaped(&mut self) -> Result<(Option<char>, bool), ParseError> {
        match self.next() {
            None => Ok((None, false)),
            Some(ch) => match ch {
                '\\' => Ok((Some(self.escape_next()?), true)),
                _ => Ok((Some(ch), false)),
            },
        }
    }

    fn escape_next(&mut self) -> Result<char, ParseError> {
        match self.next() {
            None => return Err(ParseError::MissingCharacterToEscape),
            Some(ch) => match ch {
                '(' | ')' | '{' | '}' | '[' | ']' | '|' | '\\' | '^' | '$' | '.' | '*' | '?'
                | '+' => Ok(ch),
                _ => return Err(ParseError::UnexpectedCharErr(ch)),
            },
        }
    }
}

impl Parser {
    pub fn new() -> Self {
        Parser {}
    }

    pub fn parse_str(self: &Self, input: &str) -> Result<ParseResult, ParseError> {
        let mut parser = ParserImpl {
            iter: input.chars().peekable(),
        };

        Ok(ParseResult {
            head: parser.parse(None)?,
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
    Set {
        set: Vec<char>,
        inverted: bool,
    },
    Or {
        left: Rc<RefCell<Node>>,
        right: Rc<RefCell<Node>>,
    },
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

        let parsed = parser
            .parse_str("hello world 123")
            .expect("failed to parse");
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

        let parsed = parser
            .parse_str("he(llo (1(23) wor)ld)")
            .expect("failed to parse");
        println!("{:?}", parsed);
        todo!();
    }

    #[test]
    fn test_parse_set() {
        let parser = Parser::new();

        let parsed = parser
            .parse_str("hel[^lo] (123) w[orld]")
            .expect("failed to parse");
        println!("{:?}", parsed);
        todo!();
    }

    #[test]
    fn test_parse_escaped() {
        let parser = Parser::new();

        let parsed = parser
            .parse_str("foo\\[ bar\\\\ baz\\^")
            .expect("failed to parse");

        assert_eq!(
            format!("{:?}", parsed),
            format!(
                "{:?}",
                ParseResult {
                    head: node_from_literal_str("foo[ bar\\ baz^")
                }
            ),
        )
    }
}
