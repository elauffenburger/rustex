use core::fmt;
use std::{
    cell::RefCell,
    collections::{self},
    rc::Rc,
};

use crate::parser::{Node, ParseResult};

#[derive(Debug, Clone)]
pub struct ExecResult {
    pub start: usize,
    pub end: usize,
    pub groups: collections::HashMap<String, String>,
}

pub enum ExecError {
    EmptyParseResult,
    PoisonedNode,
}

impl fmt::Debug for ExecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyParseResult => write!(f, "cannot execute empty parse result"),
            Self::PoisonedNode => write!(f, "internal: encountered poisoned node"),
        }
    }
}

pub struct Executor {}

impl Executor {
    pub fn new() -> Self {
        Executor {}
    }

    pub fn exec(
        &mut self,
        parsed: ParseResult,
        input: &str,
    ) -> Result<Option<ExecResult>, ExecError> {
        let mut executor = ExecutorImpl {
            input,
            n: input.len(),
        };

        executor.exec(None, parsed.head, 0)
    }
}

struct ExecutorImpl<'input> {
    input: &'input str,
    n: usize,
}

impl<'input> ExecutorImpl<'input> {
    fn find_word(&self, word: &str, start: usize, can_move_window: bool) -> Option<(usize, usize)> {
        let word_n = word.len();

        let mut cur = start;

        loop {
            if cur + word_n > self.n {
                return None;
            }

            let substr = &self.input[cur..cur + word_n];
            if word == substr {
                return Some((cur, cur + word_n - 1));
            }

            if !can_move_window {
                return None;
            }

            cur += 1;
        }
    }

    fn exec(
        &mut self,
        res: Option<ExecResult>,
        node: Option<Rc<RefCell<Node>>>,
        cur: usize,
    ) -> Result<Option<ExecResult>, ExecError> {
        let node = match node {
            None => match res {
                None => return Ok(res),
                Some(mut res) => {
                    res.end = cur - 1;

                    return Ok(Some(res));
                }
            },
            Some(node) => node,
        };

        let node = &node.as_ref().borrow();
        match &node.val {
            crate::parser::NodeVal::Poisoned => Err(ExecError::PoisonedNode),
            crate::parser::NodeVal::Any => self.exec(res, node.next.clone(), cur + 1),
            crate::parser::NodeVal::Start => {
                if cur == 0 {
                    self.exec(res, node.next.clone(), cur)
                } else {
                    Ok(None)
                }
            }
            crate::parser::NodeVal::End => {
                if cur == self.n {
                    self.exec(res, node.next.clone(), cur)
                } else {
                    Ok(None)
                }
            }
            crate::parser::NodeVal::Word(word) => {
                if cur >= self.n {
                    return Ok(None);
                }

                match res {
                    None => match self.find_word(word.as_str(), cur, true) {
                        None => Ok(None),
                        Some((start, end)) => self.exec(
                            Some(ExecResult {
                                groups: hashmap! {},
                                start,
                                end: 0,
                            }),
                            node.next.clone(),
                            end + 1,
                        ),
                    },
                    res @ Some(_) => match self.find_word(word.as_str(), cur, false) {
                        None => Ok(None),
                        Some((_, end)) => self.exec(res, node.next.clone(), end + 1),
                    },
                }
            }
            crate::parser::NodeVal::ZeroOrMore(_) => todo!(),
            crate::parser::NodeVal::OneOrMore(_) => todo!(),
            crate::parser::NodeVal::Optional(_) => todo!(),
            crate::parser::NodeVal::Group { .. } => todo!(),
            crate::parser::NodeVal::Set { set, inverted } => {
                let ch = match self.input.chars().nth(cur) {
                    None => todo!(),
                    Some(ch) => ch,
                };

                match (inverted, set.contains(&ch)) {
                    // not inverted, did find:
                    (false, true) | (true, false) => self.exec(res, node.next.clone(), cur + 1),
                    _ => Ok(None),
                }
            }
            crate::parser::NodeVal::Or { left, right } => {
                if let Some(left_match) = self.exec(res.clone(), Some(left.clone()), cur)? {}

                if let Some(right_match) = self.exec(res.clone(), Some(right.clone()), cur)? {}

                Ok(None)
            }
            crate::parser::NodeVal::RepetitionRange { .. } => todo!(),
        }
    }
}
