use core::fmt;
use std::{cell::RefCell, iter::Peekable, mem, rc::Rc};

mod node;
mod parse_node;

pub use node::*;
use parse_node::*;

fn rcref<T>(val: T) -> Rc<RefCell<T>> {
    Rc::new(RefCell::new(val))
}

#[derive(Debug, Clone)]
pub enum GroupConfig {
    NonCapturing,
    Named(String),
}

pub enum ParseError {
    UnexpectedCharErr(char),
    UnterminatedCharSet,
    EmptyCaptureGroup,
    MissingCharacterToEscape,
    MissingRightSideOfOr,
    BadGroupConfig,
    MissingRepetitionRangeMin,
    UnexpectedRepetitionRangeCh(char),
    MissingLeftSideOfModifier,
    UnexpectedEmptyParseNodeOption,
    ParseGraphCycle,
    UnexpectedEndOfInput,
}

impl fmt::Debug for ParseError {
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
            ParseError::MissingRepetitionRangeMin => f.write_str("missing repetition range min"),
            ParseError::UnexpectedRepetitionRangeCh(ch) => {
                f.write_fmt(format_args!("unexpected char '{}' in repetition range", ch))
            }
            ParseError::MissingLeftSideOfModifier => f.write_str("missing left side of modifier"),
            ParseError::UnexpectedEmptyParseNodeOption => {
                f.write_str("internal: unexpected unwrap of Option<Rc<RefCell<ParseNode>>>")
            }
            Self::ParseGraphCycle => write!(f, "found reference cycle in parse graph"),
            Self::UnexpectedEndOfInput => write!(f, "found unexpected end of input"),
        }
    }
}

pub struct ParseErrorWithContext<'str> {
    err: ParseError,
    str: &'str str,
    cur: usize,
}

impl<'str> fmt::Debug for ParseErrorWithContext<'str> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.err.fmt(f)?;
        f.write_fmt(format_args!(" at :{}\n", self.cur))?;
        f.write_str(self.str)?;
        f.write_str("\n")?;
        f.write_str(" ".repeat(self.cur - 1).as_str())?;
        f.write_str("^")
    }
}

pub struct Parser {}

struct ParserImpl<Iter>
where
    Iter: Iterator<Item = char>,
{
    iter: Peekable<Iter>,
    index: usize,
}

impl<Iter> ParserImpl<Iter>
where
    Iter: Iterator<Item = char>,
{
    const SPECIAL_CHARS: &[char] = &[
        '(', ')', '{', '}', '[', ']', '|', '\\', '^', '$', '.', '*', '?', '+',
    ];

    fn parse_group(&mut self) -> Result<ParseNodeVal, ParseError> {
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

        Ok(ParseNodeVal::Group {
            group,
            cfg: group_config,
        })
    }

    fn parse_repetition_range_vals(&mut self) -> Result<(u32, Option<u32>), ParseError> {
        self.next();

        let mut min_str: Option<String> = None;
        let mut max_str: Option<String> = None;

        while let Some(ch) = self.next() {
            match ch {
                '0'..='9' => {
                    if let Some(ref mut min_str) = min_str {
                        min_str.push(ch);
                    } else {
                        min_str = Some(String::from(ch));
                    }
                }
                '}' => break,
                ',' => {
                    while let Some(ch) = self.next() {
                        match ch {
                            '0'..='9' => {
                                if let Some(ref mut max_str) = max_str {
                                    max_str.push(ch);
                                } else {
                                    max_str = Some(String::from(ch));
                                }
                            }
                            '}' => break,
                            _ => return Err(ParseError::UnexpectedRepetitionRangeCh(ch)),
                        }
                    }

                    break;
                }
                _ => return Err(ParseError::UnexpectedRepetitionRangeCh(ch)),
            }
        }

        let min = min_str
            .ok_or(ParseError::MissingRepetitionRangeMin)?
            .parse::<u32>()
            .expect("should have caught bad u32 in parsing");

        let max = max_str.map(|max| {
            max.parse::<u32>()
                .expect("should have caught bad u32 in parsing")
        });

        Ok((min, max))
    }

    fn parse_word(&mut self) -> Result<ParseNodeVal, ParseError> {
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

        Ok(ParseNodeVal::Word(word))
    }

    fn is_special_char(ch: &char) -> bool {
        Self::SPECIAL_CHARS.contains(ch)
    }

    fn decorate_node_option<F: FnOnce(Rc<RefCell<ParseNode>>) -> ParseNodeVal>(
        node: &mut Option<Rc<RefCell<ParseNode>>>,
        decorator: F,
    ) -> Result<(), ParseError> {
        if node.is_none() {
            return Err(ParseError::UnexpectedEmptyParseNodeOption);
        }

        // Grab the node.
        let orig_node = mem::take(node).unwrap();

        // Grab the val of the original node.
        let orig_node_val = mem::replace(
            &mut orig_node.as_ref().borrow_mut().val,
            ParseNodeVal::Poisoned,
        );

        let res_val = decorator(rcref(ParseNode {
            val: orig_node_val,
            next: None,
        }));

        // Swap the result value into the moved node.
        orig_node.as_ref().borrow_mut().val = res_val;

        // Swap the modified node back into the node addr.
        mem::swap(node, &mut Some(orig_node));

        Ok(())
    }

    fn decorate_node_option_for_last_char_modifiers<
        F: FnOnce(Rc<RefCell<ParseNode>>) -> ParseNodeVal,
    >(
        node: &mut Option<Rc<RefCell<ParseNode>>>,
        decorator: F,
    ) -> Result<(), ParseError> {
        let take_last_ch = match node {
            Some(node) => match node.as_ref().borrow().val {
                ParseNodeVal::Word(ref word) => word.len() > 1,
                _ => false,
            },
            None => return Err(ParseError::MissingLeftSideOfModifier),
        };

        if !take_last_ch {
            return Self::decorate_node_option(node, decorator);
        }

        let orig_node = mem::take(node).unwrap();

        let orig_node_val = mem::replace(
            &mut orig_node.as_ref().borrow_mut().val,
            ParseNodeVal::Poisoned,
        );

        let (new_node_val_word, last_ch_as_str) = match orig_node_val {
            ParseNodeVal::Word(word) => {
                let mut new_node_val_word = String::new();
                let mut last_ch_as_str = None;
                let mut iter = word.chars().peekable();

                while let Some(ch) = iter.next() {
                    if iter.peek().is_none() {
                        last_ch_as_str = Some(String::from(ch));
                        break;
                    }

                    new_node_val_word.push(ch);
                }

                (
                    new_node_val_word,
                    last_ch_as_str.expect("should have found a last char in word"),
                )
            }
            _ => unreachable!("already confirmed the value is a ParseNodeVal::Word"),
        };

        let new_next = rcref(ParseNode {
            val: decorator(rcref(ParseNode {
                val: ParseNodeVal::Word(last_ch_as_str),
                next: None,
            })),
            next: None,
        });

        // Swap in the new node value and node.next (which will be swapped into the node addr).
        let mut orig_node_mut = orig_node.as_ref().borrow_mut();
        orig_node_mut.val = ParseNodeVal::Word(new_node_val_word);
        orig_node_mut.next = Some(new_next.clone());

        // Swap in the new node.
        mem::swap(node, &mut Some(new_next));

        Ok(())
    }

    fn chomp_greediness(&mut self) -> bool {
        if let Some(ch) = self.peek() {
            if *ch == '?' {
                self.next();
                return false;
            }
        };

        return true;
    }

    fn parse(
        self: &mut Self,
        until: Option<char>,
    ) -> Result<Option<Rc<RefCell<ParseNode>>>, ParseError> {
        let mut head = None;
        let mut prev: Option<Rc<RefCell<ParseNode>>> = None;

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
                    // Parse the repetition range vals.
                    let (min, max) = self.parse_repetition_range_vals()?;

                    Self::decorate_node_option_for_last_char_modifiers(&mut prev, |old_prev| {
                        ParseNodeVal::RepetitionRange {
                            min,
                            max,
                            node: old_prev,
                        }
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
                    let res_val = ParseNodeVal::Or {
                        left: left.unwrap(),
                        right,
                    };

                    let new_head = rcref(ParseNode {
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
                    let mut set = indexmap::IndexSet::new();
                    while let (Some(ch), escaped) = self.next_escaped()? {
                        if !escaped && ch == ']' {
                            found_end = true;
                            break;
                        }

                        set.insert(ch);
                    }

                    if !found_end {
                        return Err(ParseError::UnterminatedCharSet);
                    }

                    ParseNodeVal::Set { set, inverted }
                }
                '.' => {
                    self.next();

                    ParseNodeVal::Any
                }
                '*' => {
                    self.next();

                    let greedy = self.chomp_greediness();
                    Self::decorate_node_option_for_last_char_modifiers(&mut prev, |old_node| {
                        ParseNodeVal::ZeroOrMore {
                            node: old_node,
                            greedy,
                        }
                    })?;

                    continue;
                }
                '+' => {
                    self.next();

                    let greedy = self.chomp_greediness();
                    Self::decorate_node_option_for_last_char_modifiers(&mut prev, |old_node| {
                        ParseNodeVal::OneOrMore {
                            node: old_node,
                            greedy,
                        }
                    })?;

                    continue;
                }
                '?' => {
                    self.next();

                    Self::decorate_node_option_for_last_char_modifiers(&mut prev, |old_node| {
                        ParseNodeVal::Optional(old_node)
                    })?;

                    continue;
                }
                '^' => {
                    self.next();

                    ParseNodeVal::Start
                }
                '$' => {
                    self.next();

                    ParseNodeVal::End
                }
                '(' => self.parse_group()?,
                _ => self.parse_word()?,
            };

            let new_node = rcref(ParseNode {
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
        self.index += 1;
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

    pub fn parse_str<'str>(
        self: &Self,
        input: &'str str,
    ) -> Result<ParseResult, ParseErrorWithContext<'str>> {
        let mut parser = ParserImpl {
            iter: input.chars().peekable(),
            index: 0,
        };

        Ok(ParseResult {
            head: parser
                .parse(None)
                .and_then(|head| match head {
                    None => Ok(None),
                    Some(head) => {
                        let head = Rc::try_unwrap(head)
                            .map_err(|_| ParseError::ParseGraphCycle)
                            .map(|head| head.into_inner())?;

                        Ok(Some(Rc::new(head.try_into()?)))
                    }
                })
                .map_err(|err| ParseErrorWithContext {
                    err,
                    str: input,
                    cur: parser.index,
                })?,
        })
    }
}

#[derive(Default, Clone)]
pub struct ParseResult {
    pub head: Option<Rc<Node>>,
}

impl fmt::Debug for ParseResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ParseResult { ")?;
        match &self.head {
            None => {}
            Some(head) => {
                head.fmt(f)?;
            }
        }
        f.write_str(" }")
    }
}
