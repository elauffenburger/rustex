use core::fmt;

use crate::parser::ParseResult;

mod node;
use node::*;

#[derive(Debug, Clone)]
pub struct ExecResult {
    pub start: usize,
    pub end: usize,
    pub groups: indexmap::IndexMap<String, (usize, usize)>,
}

impl ExecResult {
    pub fn new(start: usize) -> Self {
        Self {
            start,
            end: 0,
            groups: indexmap::indexmap! {},
        }
    }

    fn merge(&mut self, other: ExecResult) {
        self.groups.extend(other.groups);
    }

    fn map_options(dest: Option<Self>, src: Option<Self>) -> Option<Self> {
        match (dest, src) {
            (None, None) => None,
            (None, src @ Some(_)) => src,
            (dest @ Some(_), None) => dest,
            (Some(mut src), Some(dest)) => {
                src.merge(dest);

                Some(src)
            }
        }
    }
}

pub enum ExecError {
    EmptyParseResult,
    PoisonedNode,
    ParseGraphCycle,
}

impl fmt::Debug for ExecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyParseResult => write!(f, "cannot execute empty parse result"),
            Self::PoisonedNode => write!(f, "internal: encountered poisoned node"),
            Self::ParseGraphCycle => write!(f, "found reference cycle in parse graph"),
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

        match parsed.head {
            None => Ok(None),
            Some(head) => executor.exec(None, Some(&ExecNode::from_parsed(head)?), 0),
        }
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

    fn exec_repeated(
        &mut self,
        res: Option<ExecResult>,
        node: Option<&Box<ExecNode>>,
        cur: usize,
        abort_after: Option<u32>,
    ) -> Result<(Option<ExecResult>, usize, u32, bool), ExecError> {
        let to_test = node;
        let mut res = res;
        let mut cur = cur;

        // Try to exec to_test as many times as we can.
        let mut num_execs = 0;
        while let Some(exec_res) = self.exec(res.clone(), to_test, cur)? {
            num_execs += 1;
            cur = exec_res.end + 1;
            res = ExecResult::map_options(res, Some(exec_res));

            // Check if we should abort execution.
            if let Some(abort_after) = abort_after {
                if num_execs == abort_after {
                    return Ok((res, cur, num_execs, true));
                }
            }
        }

        Ok((res, cur, num_execs, false))
    }

    fn exec(
        &mut self,
        res: Option<ExecResult>,
        node: Option<&Box<ExecNode>>,
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

        match &node.val {
            ExecNodeVal::Poisoned => Err(ExecError::PoisonedNode),
            ExecNodeVal::Any => self.exec(res, node.next.as_ref(), cur + 1),
            ExecNodeVal::Start => {
                if cur == 0 {
                    self.exec(res, node.next.as_ref(), cur)
                } else {
                    Ok(None)
                }
            }
            ExecNodeVal::End => {
                if cur == self.n {
                    self.exec(res, node.next.as_ref(), cur)
                } else {
                    Ok(None)
                }
            }
            ExecNodeVal::Word(word) => {
                if cur >= self.n {
                    return Ok(None);
                }

                match res {
                    None => match self.find_word(word.as_str(), cur, true) {
                        None => Ok(None),
                        Some((start, end)) => {
                            self.exec(Some(ExecResult::new(start)), node.next.as_ref(), end + 1)
                        }
                    },
                    res @ Some(_) => match self.find_word(word.as_str(), cur, false) {
                        None => Ok(None),
                        Some((_, end)) => self.exec(res, node.next.as_ref(), end + 1),
                    },
                }
            }
            ExecNodeVal::ZeroOrMore(to_test) => {
                let (res, new_cur, _, _) = self.exec_repeated(res, Some(to_test), cur, None)?;

                self.exec(res, node.next.as_ref(), new_cur)
            }
            ExecNodeVal::OneOrMore(to_test) => {
                let (res, new_cur, num_execs, _) =
                    self.exec_repeated(res, Some(to_test), cur, None)?;
                if num_execs == 0 {
                    return Ok(None);
                }

                self.exec(res, node.next.as_ref(), new_cur)
            }
            ExecNodeVal::RepetitionRange {
                node: to_test,
                min,
                max,
            } => {
                let (res, new_cur, num_execs, _) =
                    self.exec_repeated(res, Some(to_test), cur, *max)?;
                if num_execs < *min {
                    return Ok(None);
                }

                self.exec(res, node.next.as_ref(), new_cur)
            }
            ExecNodeVal::Optional(to_test) => {
                // Attempt to match at most one time, but continue on if we didn't match.
                let (res, new_cur, _, _) = self.exec_repeated(res, Some(to_test), cur, Some(1))?;

                self.exec(res, node.next.as_ref(), new_cur)
            }
            ExecNodeVal::Group {
                group,
                cfg: group_cfg,
            } => match self.exec(res.clone(), Some(group), cur)? {
                None => return Ok(None),
                Some(exec_res) => {
                    let new_cur = exec_res.end;
                    let mut res = ExecResult::map_options(res, Some(exec_res));

                    // Record this group if needed.
                    match (&mut res, group_cfg) {
                        (Some(res), Some(group_cfg)) => match group_cfg {
                            crate::parser::GroupConfig::NonCapturing => {}
                            crate::parser::GroupConfig::Named(name) => {
                                res.groups.insert(name.clone(), (cur, new_cur));
                            }
                        },
                        _ => {}
                    }

                    self.exec(res, node.next.as_ref(), new_cur + 1)
                }
            },
            ExecNodeVal::Set { set, inverted } => {
                let ch = match self.input.chars().nth(cur) {
                    None => todo!(),
                    Some(ch) => ch,
                };

                match (inverted, set.contains(&ch)) {
                    // not inverted, did find:
                    (false, true) | (true, false) => {
                        let mut res = res;
                        if let None = res {
                            res = Some(ExecResult::new(cur));
                        }

                        self.exec(res, node.next.as_ref(), cur + 1)
                    }
                    _ => Ok(None),
                }
            }
            ExecNodeVal::Or { left, right } => {
                let match_result = match self.exec(res.clone(), Some(left), cur)? {
                    None => match self.exec(res.clone(), Some(right), cur)? {
                        None => None,
                        res @ Some(_) => res,
                    },
                    res @ Some(_) => res,
                };

                match match_result {
                    None => Ok(None),
                    Some(match_result) => {
                        let new_res = if let Some(mut res) = res {
                            res.merge(match_result);

                            res
                        } else {
                            match_result
                        };

                        let new_end = new_res.end;
                        self.exec(Some(new_res), node.next.as_ref(), cur + new_end)
                    }
                }
            }
        }
    }
}
