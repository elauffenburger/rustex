use core::fmt;
use std::{cell::RefCell, error::Error, iter::Peekable, mem, rc::Rc};

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
    UnexpectedRepititionRangeCh(char),
    MissingLeftSideOfModifier,
    UnexpectedEmptyNodeOption,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::UnexpectedCharErr(ch) => {
                f.write_fmt(format_args!("unexpected char '{}'", ch))
            }
            ParseError::UnterminatedCharSet => f.write_str("unterminated character set"),
            ParseError::EmptyCaptureGroup => f.write_str("empty capture group"),
            ParseError::MissingCharacterToEscape => f.write_str("missing character to escape"),
            ParseError::MissingRightSideOfOr => f.write_str("missing right side of or"),
            ParseError::BadGroupConfig => f.write_str("bad group config"),
            ParseError::UnexpectedRepititionRangeCh(ch) => {
                f.write_fmt(format_args!("unexpected char '{}' in repitition range", ch))
            }
            ParseError::MissingLeftSideOfModifier => f.write_str("missing left side of modifier"),
            ParseError::UnexpectedEmptyNodeOption => {
                f.write_str("internal: unexpected unwrap of Option<Rc<RefCell<Node>>>")
            }
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
                self.next();
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

    fn parse_repitition_range_vals(&mut self) -> Result<(u32, Option<u32>), ParseError> {
        self.next();

        let mut min_str = String::new();
        let mut max_str = None;

        while let Some(ch) = self.next() {
            match ch {
                '0'..='9' => {
                    min_str.push(ch);
                }
                '}' => break,
                ',' => {
                    let mut max_str_local = String::new();
                    while let Some(ch) = self.next() {
                        match ch {
                            '0'..='9' => {
                                max_str_local.push(ch);
                            }
                            '}' => break,
                            _ => return Err(ParseError::UnexpectedRepititionRangeCh(ch)),
                        }
                    }

                    max_str = Some(max_str_local);
                    break;
                }
                _ => return Err(ParseError::UnexpectedRepititionRangeCh(ch)),
            }
        }

        let min = min_str
            .parse::<u32>()
            .expect("should have caught bad u32 in parsing");

        let max = max_str.map(|max| {
            max.parse::<u32>()
                .expect("should have caught bad u32 in parsing")
        });

        Ok((min, max))
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

    fn decorate_node<F: FnOnce(Rc<RefCell<Node>>) -> NodeVal>(
        node: &mut Rc<RefCell<Node>>,
        decorator: F,
    ) {
        // Grab the node.
        let mut orig_node = rcref(Node {
            val: NodeVal::Any,
            next: None,
        });
        mem::swap(&mut orig_node, node);

        // Grab the val of the original node.
        let mut orig_node_val = NodeVal::Any;
        mem::swap(&mut orig_node_val, &mut orig_node.as_ref().borrow_mut().val);

        let res_val = decorator(rcref(Node {
            val: orig_node_val,
            next: None,
        }));

        // Swap the result value into the moved node.
        orig_node.as_ref().borrow_mut().val = res_val;

        // Swap the modified node back into the original node addr.
        mem::swap(node, &mut orig_node);
    }

    fn decorate_node_option<F: FnOnce(Rc<RefCell<Node>>) -> NodeVal>(
        node: &mut Option<Rc<RefCell<Node>>>,
        decorator: F,
    ) -> Result<(), ParseError> {
        if node.is_none() {
            return Err(ParseError::UnexpectedEmptyNodeOption);
        }

        // Grab the node.
        let mut orig_node = mem::take(node).unwrap();
        Self::decorate_node(&mut orig_node, decorator);

        // Swap the modified node back into the node addr.
        mem::swap(node, &mut Some(orig_node));

        Ok(())
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
                '{' => {
                    // Parse the repitition range vals.
                    let (min, max) = self.parse_repitition_range_vals()?;

                    Self::decorate_node_option(&mut prev, |old_prev| NodeVal::RepititionRange {
                        min,
                        max,
                        node: old_prev,
                    })?;

                    continue;
                }
                '|' => {
                    self.next();

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
                    self.next();

                    let mut inverted = false;
                    if let Some(next) = self.peek() {
                        if *next == '^' {
                            inverted = true;
                            self.next();
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
                '.' => {
                    self.next();

                    NodeVal::Any
                }
                '*' => {
                    self.next();

                    // TODO: only repeat last char in word.

                    let take_last_ch = match &prev {
                        Some(node) => match node.as_ref().borrow().val {
                            NodeVal::Word(ref word) => word.len() > 1,
                            _ => false,
                        },
                        None => return Err(ParseError::MissingLeftSideOfModifier),
                    };

                    if take_last_ch {
                        let orig_prev = mem::take(&mut prev).unwrap();

                        let mut orig_prev_val = NodeVal::Any;
                        mem::swap(&mut orig_prev.as_ref().borrow_mut().val, &mut orig_prev_val);

                        let (new_prev_val_word, last_ch_as_str) = match orig_prev_val {
                            NodeVal::Word(word) => {
                                let mut new_prev_val_word = String::new();
                                let mut last_ch_as_str = None;
                                let mut iter = word.chars().peekable();

                                while let Some(ch) = iter.next() {
                                    if iter.peek().is_none() {
                                        last_ch_as_str = Some(String::from(ch));
                                        break;
                                    }

                                    new_prev_val_word.push(ch);
                                }

                                (
                                    new_prev_val_word,
                                    last_ch_as_str.expect("should have found a last char in word"),
                                )
                            }
                            _ => unreachable!("already confirmed the value is a NodeVal::Word"),
                        };

                        let new_next = rcref(Node {
                            val: NodeVal::ZeroOrMore(rcref(Node {
                                val: NodeVal::Word(last_ch_as_str),
                                next: None,
                            })),
                            next: None,
                        });

                        // Swap in the new prev value and prev.next (which will become prev).
                        let mut orig_prev_mut = orig_prev.as_ref().borrow_mut();
                        orig_prev_mut.val = NodeVal::Word(new_prev_val_word);
                        orig_prev_mut.next = Some(new_next.clone());

                        // Swap in the new prev.
                        mem::swap(&mut prev, &mut Some(new_next));
                    } else {
                        Self::decorate_node_option(&mut prev, |old_prev| {
                            NodeVal::ZeroOrMore(old_prev)
                        })?;
                    }

                    continue;
                }
                '+' => {
                    self.next();

                    // TODO: only repeat last char in word.

                    Self::decorate_node_option(&mut prev, |old_prev| NodeVal::OneOrMore(old_prev))?;
                    continue;
                }
                '?' => {
                    self.next();

                    // TODO: only repeat last char in word.

                    Self::decorate_node_option(&mut prev, |old_prev| NodeVal::Optional(old_prev))?;
                    continue;
                }
                '^' => {
                    self.next();

                    NodeVal::Start
                }
                '$' => {
                    self.next();

                    NodeVal::End
                }
                '(' => self.parse_group()?,
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
