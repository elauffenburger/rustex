use core::fmt;
use std::{cell::RefCell, iter::Peekable, mem, rc::Rc};

mod node;
pub use node::*;

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
    const SPECIAL_CHARS: &[char] = &[
        '(', ')', '{', '}', '[', ']', '|', '\\', '^', '$', '.', '*', '?', '+',
    ];

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

    fn parse_word(&mut self) -> Result<NodeVal, ParseError> {
        let mut word = String::new();
        while let Some(ch) = self.iter.peek() {
            if Self::is_special_char(ch) && *ch != '\\' {
                break;
            }

            let mut ch = self.next().unwrap();
            if ch == '\\' {
                ch = self.escape_next()?;
            }

            word.push(ch);
        }

        Ok(NodeVal::Word(word))
    }

    fn is_special_char(ch: &char) -> bool {
        Self::SPECIAL_CHARS.contains(ch)
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
                '(' => self.parse_group()?,
                '|' => {
                    _ = self.next();

                    // Grab the head of the current parse group and consider everything under it the left side.
                    let left = mem::take(&mut head);

                    // Parse everything after the "or" as a separate group and consider it the right side.
                    let right = match self.parse(Some(')'))? {
                        None => return Err(ParseError::MissingRightSideOfOr),
                        Some(right) => right,
                    };

                    // Construct the result.
                    let res_val = NodeVal::Or {
                        left: left.unwrap(),
                        right,
                    };

                    let new_head = rcref(Node {
                        val: res_val.clone(),
                        next: None,
                    });
                    mem::swap(&mut head, &mut Some(new_head.clone()));
                    mem::swap(&mut prev, &mut Some(new_head));

                    continue;
                }
                '[' => {
                    _ = self.next();

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
                '.' => NodeVal::Any,
                '*' => NodeVal::ZeroOrMore,
                '+' => NodeVal::OneOrMore,
                '?' => NodeVal::Optional,
                '^' => NodeVal::Start,
                '$' => NodeVal::End,
                _ => self.parse_word()?,
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
