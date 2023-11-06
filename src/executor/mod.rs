use core::fmt;
use std::rc::Rc;

use crate::parser::{self, Node, NodeVal};

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
        parsed: parser::ParseResult,
        input: &str,
    ) -> Result<Option<ExecResult>, ExecError> {
        let mut executor = ExecutorImpl {
            input,
            n: input.len(),
        };

        let head: Option<Rc<Node>> = parsed.head.map(|head| Rc::from(head));
        executor.exec(None, head.as_ref(), None, 0)
    }
}

struct ExecutorImpl<'input> {
    input: &'input str,
    n: usize,
}

impl<'input> ExecutorImpl<'input> {
    fn next(node: Option<&Rc<Node>>) -> Option<&Rc<Node>> {
        match node {
            None => None,
            Some(node) => match &node.as_ref().next {
                None => None,
                Some(next) => Some(next),
            },
        }
    }

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
        node: Option<&Rc<Node>>,
        in_group: Option<&Rc<Node>>,
        cur: usize,
        abort_after: Option<u32>,
        abort_if_match: Option<&Rc<Node>>,
    ) -> Result<(Option<ExecResult>, usize, u32, bool), ExecError> {
        let to_test = node;
        let mut res = res;
        let mut cur = cur;

        // Try to exec to_test as many times as we can.
        let mut num_execs = 0;
        while let Some(exec_res) = self.exec(res.clone(), to_test, in_group, cur)? {
            num_execs += 1;
            cur = exec_res.end + 1;
            res = ExecResult::map_options(res, Some(exec_res));

            // Check if we reached the max execs.
            if let Some(abort_after) = abort_after {
                if num_execs == abort_after {
                    return Ok((res, cur, num_execs, true));
                }
            }

            // Check if we need to abort because the next node matches.
            if abort_if_match.is_some() {
                if let Some(_) = self.exec(
                    res.clone(),
                    abort_if_match
                        .map(|node| {
                            Rc::new(Node {
                                val: node.val.clone(),
                                next: None,
                            })
                        })
                        .as_ref(),
                    None,
                    cur,
                )? {
                    return Ok((res, cur, num_execs, true));
                }
            }
        }

        Ok((res, cur, num_execs, false))
    }

    fn exec(
        &mut self,
        res: Option<ExecResult>,
        node: Option<&Rc<Node>>,
        in_group: Option<&Rc<Node>>,
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
            NodeVal::Poisoned => Err(ExecError::PoisonedNode),
            NodeVal::Any => {
                if cur == self.n {
                    return Ok(None);
                }

                self.exec(
                    res.or(Some(ExecResult::new(cur))),
                    node.next.as_ref(),
                    in_group,
                    cur + 1,
                )
            }
            NodeVal::Start => {
                if cur == 0 {
                    self.exec(
                        res.or(Some(ExecResult::new(cur))),
                        node.next.as_ref(),
                        in_group,
                        cur,
                    )
                } else {
                    Ok(None)
                }
            }
            NodeVal::End => {
                if cur == self.n {
                    return self.exec(
                        res.or(Some(ExecResult::new(cur))),
                        node.next.as_ref(),
                        in_group,
                        cur,
                    );
                }

                Ok(None)
            }
            NodeVal::Word(word) => {
                if cur >= self.n {
                    return Ok(None);
                }

                match self.find_word(word, cur, res.is_none()) {
                    None => Ok(None),
                    Some((start, end)) => self.exec(
                        res.or(Some(ExecResult::new(start))),
                        node.next.as_ref(),
                        in_group,
                        end + 1,
                    ),
                }
            }
            NodeVal::ZeroOrMore {
                node: to_test,
                greedy,
            } => {
                let (res, new_cur, _, _) = self.exec_repeated(
                    res,
                    Some(to_test),
                    in_group,
                    cur,
                    None,
                    if !*greedy {
                        Self::next(in_group).or(Self::next(Some(node)))
                    } else {
                        None
                    },
                )?;

                self.exec(res, node.next.as_ref(), in_group, new_cur)
            }
            NodeVal::OneOrMore {
                node: to_test,
                greedy,
            } => {
                let (res, new_cur, num_execs, _) = self.exec_repeated(
                    res,
                    Some(to_test),
                    in_group,
                    cur,
                    None,
                    if !*greedy {
                        Self::next(in_group).or(Self::next(Some(node)))
                    } else {
                        None
                    },
                )?;
                if num_execs == 0 {
                    return Ok(None);
                }

                self.exec(res, node.next.as_ref(), in_group, new_cur)
            }
            NodeVal::RepetitionRange {
                node: to_test,
                min,
                max,
            } => {
                let (res, new_cur, num_execs, _) =
                    self.exec_repeated(res, Some(to_test), in_group, cur, *max, None)?;
                if num_execs < *min {
                    return Ok(None);
                }

                self.exec(res, node.next.as_ref(), in_group, new_cur)
            }
            NodeVal::Optional(to_test) => {
                // Attempt to match at most one time, but continue on if we didn't match.
                let (res, new_cur, _, _) =
                    self.exec_repeated(res, Some(to_test), in_group, cur, Some(1), None)?;

                self.exec(res, node.next.as_ref(), in_group, new_cur)
            }
            NodeVal::Group {
                group,
                cfg: group_cfg,
            } => match self.exec(res.clone(), Some(group), Some(node), cur)? {
                None => return Ok(None),
                Some(exec_res) => {
                    let new_cur = exec_res.end;
                    let mut res = ExecResult::map_options(res, Some(exec_res));

                    // Record this group if needed.
                    match (&mut res, group_cfg) {
                        (Some(res), Some(group_cfg)) => match group_cfg {
                            parser::GroupConfig::NonCapturing => {}
                            parser::GroupConfig::Named(name) => {
                                res.groups.insert(name.clone(), (cur, new_cur));
                            }
                        },
                        _ => {}
                    }

                    self.exec(
                        res,
                        node.next.as_ref(),
                        Self::next(node.next.as_ref()),
                        new_cur + 1,
                    )
                }
            },
            NodeVal::Set { set, inverted } => {
                let ch = match self.input.chars().nth(cur) {
                    None => todo!(),
                    Some(ch) => ch,
                };

                match (inverted, set.contains(&ch)) {
                    // not inverted, did find:
                    (false, true) | (true, false) => self.exec(
                        res.or(Some(ExecResult::new(cur))),
                        node.next.as_ref(),
                        in_group,
                        cur + 1,
                    ),
                    _ => Ok(None),
                }
            }
            NodeVal::Or { left, right } => {
                let match_result =
                    match self.exec(res.clone(), Some(left), Self::next(Some(left)), cur)? {
                        None => match self.exec(res.clone(), Some(right), in_group, cur)? {
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
                        self.exec(Some(new_res), node.next.as_ref(), in_group, cur + new_end)
                    }
                }
            }
        }
    }
}
