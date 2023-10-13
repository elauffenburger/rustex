use core::fmt;
use std::{cell::RefCell, iter::Peekable, mem, rc::Rc};

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
    BadGroupConfig,
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
            ParseError::BadGroupConfig => f.write_str("bad group config"),
        }
    }
}

pub struct Parser {}

#[derive(Debug, Clone)]
pub enum GroupConfig {
    NonCapturing,
    Named(String),
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
    fn parse_group(&mut self) -> Result<NodeVal, ParseError> {
        self.next();

        let group_config = match self.peek() {
            Some('?') => {
                let _ = self.next();
                match self.next() {
                    Some(':') => Some(GroupConfig::NonCapturing),
                    Some('<') => {
                        let mut name = String::new();
                        while let (Some(ch), escaped) = self.next_escaped()? {
                            if !escaped && ch == '>' {
                                break;
                            }

                            name.push(ch);
                        }

                        Some(GroupConfig::Named(name))
                    }
                    _ => return Err(ParseError::BadGroupConfig),
                }
            }
            _ => None,
        };

        let group = match self.parse(Some(')'))? {
            Some(group) => group,
            None => return Err(ParseError::EmptyCaptureGroup),
        };

        self.next();

        Ok(NodeVal::Group {
            group,
            cfg: group_config,
        })
    }

    fn parse(
        self: &mut Self,
        until: Option<char>,
    ) -> Result<Option<Rc<RefCell<Node>>>, ParseError> {
        let mut head = None;
        let mut prev: Option<Rc<RefCell<Node>>> = None;

        while let Some(ch) = self.peek() {
            match until {
                Some(until) => {
                    if *ch == until {
                        break;
                    }
                }
                None => {}
            }

            let new_node_val = match ch {
                '{' => todo!(),
                '(' => parse_group()?,
                '|' => {
                    // Grab the head of the current parse group and consider everything under it the left side.
                    let left = mem::take(&mut head);

                    // Parse everything after the "or" as a separate group and consider it the right side.
                    let right = match self.parse(None)? {
                        None => return Err(ParseError::MissingRightSideOfOr),
                        Some(right) => right,
                    };

                    // Construct the result.
                    let res_val = NodeVal::Or {
                        left: left.unwrap(),
                        right,
                    };

                    // Swap the or'd result back in as the head and prev node.
                    let new_head = rcref(Node {
                        val: res_val.clone(),
                        next: None,
                    });
                    mem::swap(&mut head, &mut Some(new_head.clone()));
                    mem::swap(&mut prev, &mut Some(new_head));

                    res_val
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

                    NodeVal::Set { set, inverted }
                }
                '\\' => NodeVal::Char(self.escape_next()?),
                '.' => NodeVal::Any,
                '*' => NodeVal::ZeroOrMore,
                '+' => NodeVal::OneOrMore,
                '?' => NodeVal::Optional,
                '^' => NodeVal::Start,
                '$' => NodeVal::End,
                _ => NodeVal::Char(ch),
            };

            let new_node = rcref(Node {
                val: new_node_val,
                next: None,
            });

            if prev.is_none() {
                head = Some(new_node.clone());
                prev = Some(new_node);
            } else {
                // Update node.next to point to the new node.
                let node_val = mem::take(&mut prev).unwrap();
                (*node_val).borrow_mut().next = Some(new_node.clone());

                // Update node to point to the new node.
                prev = Some(new_node);
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

#[derive(Default)]
pub struct ParseResult {
    pub head: Option<Rc<RefCell<Node>>>,
}

impl fmt::Debug for ParseResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ParseResult { ")?;
        match &self.head {
            None => {}
            Some(head) => {
                head.as_ref().borrow().fmt(f)?;
            }
        }
        f.write_str(" }")
    }
}

pub struct Node {
    pub val: NodeVal,
    pub next: Option<Rc<RefCell<Node>>>,
}

impl Node {
    fn fmt_internal(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.val {
            NodeVal::Char(ch) => f.write_fmt(format_args!("'{}'", ch)),
            NodeVal::Any => f.write_str("."),
            NodeVal::ZeroOrMore => f.write_str("*"),
            NodeVal::OneOrMore => f.write_str("+"),
            NodeVal::Start => f.write_str("^"),
            NodeVal::End => f.write_str("$"),
            NodeVal::Optional => f.write_str("?"),
            NodeVal::Group { group, cfg } => {
                f.write_str("(")?;

                match cfg {
                    None => {}
                    Some(GroupConfig::Named(name)) => {
                        f.write_fmt(format_args!("<{}>", name))?;
                    }
                    Some(GroupConfig::NonCapturing) => {
                        f.write_str("?:")?;
                    }
                }

                group.as_ref().borrow().fmt_internal(f)?;

                f.write_str(")")
            }
            NodeVal::Set { set, inverted } => {
                f.write_str("[")?;
                if *inverted {
                    f.write_str("^")?;
                }

                {
                    let mut iter = set.iter().peekable();
                    while let Some(ch) = iter.next() {
                        f.write_fmt(format_args!("'{}'", ch))?;

                        if let Some(_) = iter.peek() {
                            f.write_str(", ")?;
                        }
                    }
                }

                f.write_str("]")
            }
            NodeVal::Or { left, right } => {
                f.write_str("or(l(")?;
                left.as_ref().borrow().fmt_internal(f)?;
                f.write_str("), r(")?;
                right.as_ref().borrow().fmt_internal(f)?;
                f.write_str(")")
            }
        }?;

        match &self.next {
            None => Ok(()),
            Some(node) => {
                f.write_str("->")?;
                node.as_ref().borrow().fmt_internal(f)
            }
        }
    }
}

impl fmt::Debug for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_internal(f)
    }
}

#[derive(Debug, Clone)]
pub enum NodeVal {
    Char(char),
    Any,
    ZeroOrMore,
    OneOrMore,
    Start,
    End,
    Optional,
    Group {
        group: Rc<RefCell<Node>>,
        cfg: Option<GroupConfig>,
    },
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
                res = Some(new_node);
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
    fn test_parse_groups() {
        let parser = Parser::new();

        let parsed = parser
            .parse_str("he(llo (1(23) wor)ld i am (?<my_group>named) (?:unnamed) groups)")
            .expect("failed to parse");
        println!("{:?}", parsed);
        todo!();
    }

    #[test]
    fn test_parse_sets() {
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

    #[test]
    fn test_parse_or() {
        let parser = Parser::new();

        let parsed = parser
            .parse_str("(foo|(bar))ab|c")
            .expect("failed to parse");
        println!("{:?}", parsed);
        todo!();
    }
}
