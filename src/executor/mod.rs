use core::fmt;
use std::{cell::RefCell, collections::VecDeque, rc::Rc};

use crate::parser::{self, MutNode, Node, NodeVal};

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

    fn merge_groups(mut self, other: ExecResult) -> Self {
        self.groups.extend(other.groups);
        self
    }

    fn map_options(dest: Option<Self>, src: Option<Self>) -> Option<Self> {
        match (dest, src) {
            (None, None) => None,
            (None, src @ Some(_)) => src,
            (dest @ Some(_), None) => dest,
            (Some(src), Some(dest)) => Some(src.merge_groups(dest)),
        }
    }
}

pub enum ExecError {
    EmptyParseResult,
    PoisonedNode,
}

impl ExecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyParseResult => write!(f, "cannot execute empty parse result"),
            Self::PoisonedNode => write!(f, "internal: encountered poisoned node"),
        }
    }
}

impl fmt::Display for ExecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt(f)
    }
}

impl fmt::Debug for ExecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt(f)
    }
}

impl std::error::Error for ExecError {}

pub struct Executor {}

impl Executor {
    pub fn new() -> Self {
        Executor {}
    }

    pub fn exec(&mut self, parsed: &parser::ParseResult, input: &str) -> Result<Option<ExecResult>, ExecError> {
        let head: Option<Rc<Node>> = parsed.head.clone().map(|head| Rc::from(head));

        let mut executor = ExecutorImpl {
            input,
            n: input.len(),
            frontier: VecDeque::new(),
        };

        executor.frontier.push_front(ExecutorState {
            res: None,
            node: head,
            cur: 0,
        });

        let mut best_match = None;
        while let Some(state) = executor.frontier.pop_front() {
            dbg!("popped new state", &state);

            if let Some(res) = executor.exec(state.res, state.node.as_ref(), state.cur)? {
                match &best_match {
                    None => best_match = Some(res),
                    Some(curr_best) => {
                        if res.end > curr_best.end {
                            best_match = Some(res)
                        }
                    }
                }
            }
        }

        Ok(best_match)
    }
}

#[derive(Debug, Clone)]
struct ExecutorState {
    res: Option<ExecResult>,
    node: Option<Rc<Node>>,
    cur: usize,
}

struct ExecutorImpl<'input> {
    input: &'input str,
    n: usize,

    frontier: VecDeque<ExecutorState>,
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
        node: Option<&Rc<Node>>,
        cur: usize,
    ) -> Result<Option<ExecResult>, ExecError> {
        dbg!(&node);
        dbg!(cur);

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

                self.exec(res.or(Some(ExecResult::new(cur))), node.next.as_ref(), cur + 1)
            }
            NodeVal::Start => {
                if cur == 0 {
                    self.exec(res.or(Some(ExecResult::new(cur))), node.next.as_ref(), cur)
                } else {
                    Ok(None)
                }
            }
            NodeVal::End => {
                if cur == self.n {
                    return self.exec(res.or(Some(ExecResult::new(cur))), node.next.as_ref(), cur);
                }

                Ok(None)
            }
            NodeVal::Word(word) => {
                if cur >= self.n {
                    return Ok(None);
                }

                println!("dbg: looking for {:?} from {:?}", word, self.input[cur..].to_string());

                match self.find_word(word, cur, res.is_none()) {
                    None => {
                        println!("dbg: no match!");
                        Ok(None)
                    }
                    Some((start, end)) => {
                        println!("dbg: matched!");
                        self.exec(res.or(Some(ExecResult::new(start))), node.next.as_ref(), end + 1)
                    }
                }
            }
            NodeVal::Optional(to_test) => {
                // Branch the expression into two versions: one that has this node and one that doesn't and add both to the frontier.

                // Branch the "skip this node" case.
                self.frontier.push_front(ExecutorState {
                    res: res.clone(),
                    node: node.next.clone(),
                    cur,
                });
                println!("dbg: optional - skip this node state:");
                dbg!(self.frontier.front());

                // Branch the "take this node" case.
                self.frontier.push_front(ExecutorState {
                    res: res.clone(),
                    node: Some(
                        to_test
                            .as_ref()
                            .clone()
                            .into_mut()
                            .append_option_node(&node.next)
                            .into_node()
                            .into(),
                    ),
                    cur,
                });
                println!("dbg: optional - take this node state:");
                dbg!(self.frontier.front());

                // Bail and let the executor explore these states in the frontier later.
                return Ok(res);
            }
            NodeVal::ZeroOrMore { node: to_test, greedy } => {
                // TODO: make this work!

                // Branch the expression into two versions: one that matches one ore more times and one that doesn't contain the node and add both to the frontier.

                // Branch the "skip this node" case.
                self.frontier.push_front(ExecutorState {
                    res: res.clone(),
                    node: node.next.clone(),
                    cur,
                });
                println!("dbg: * - skip this node state");
                dbg!(self.frontier.front());

                // Branch the "one-or-more" case.
                self.frontier.push_front(ExecutorState {
                    res: res.clone(),
                    node: Some(Rc::new(Node {
                        val: NodeVal::OneOrMore {
                            node: to_test.clone(),
                            greedy: *greedy,
                        },
                        next: node.next.clone(),
                    })),
                    cur,
                });
                println!("dbg: * - take this node state");
                dbg!(self.frontier.front());

                // Bail and let the executor explore these states in the frontier later.
                return Ok(res);
            }
            NodeVal::OneOrMore { node: to_test, greedy } => {
                // TODO: make this work!

                let mut res = res;
                let mut cur = cur;

                println!("dbg: looking for + match");
                dbg!(cur);

                // Match at least once.
                let new_res = self.exec(res.clone(), Some(to_test), cur)?;
                res = ExecResult::map_options(res, new_res.clone());
                match new_res {
                    None => {
                        println!("dbg: failed to match");
                        return Ok(None);
                    }
                    Some(new_res) => {
                        cur = new_res.end + 1;

                        println!("dbg: matched!");
                        dbg!(cur);
                    }
                }

                // If lazy and we can match the next node, we're done!
                if !*greedy {
                    println!("dbg: lazy matching...");
                    let res = self.exec(res.clone(), node.next.clone().as_ref(), cur)?;
                    if res.is_some() {
                        println!("dbg: lazy matched; done!");
                        return Ok(res);
                    }
                }

                // We've branched our minimum number, so now we need to either keep matching or give up; we can model that with a "zero-or-more" state!
                self.frontier.push_front(ExecutorState {
                    res,
                    node: Some(Rc::new(Node {
                        val: NodeVal::ZeroOrMore {
                            node: to_test.clone(),
                            greedy: *greedy,
                        },
                        next: node.next.clone(),
                    })),
                    cur,
                });
                dbg!(self.frontier.front());

                return Ok(None);
            }
            NodeVal::RepetitionRange {
                node: to_test,
                min,
                max,
            } => {
                // TODO: make this work!

                let mut res = res;
                let mut cur = cur;

                // Match min times.
                let mut i = 0;
                loop {
                    if i == *min {
                        break;
                    }

                    let new_res = self.exec(res.clone(), Some(to_test), cur)?;
                    res = ExecResult::map_options(res, new_res.clone());
                    match new_res {
                        None => return Ok(None),
                        Some(new_res) => {
                            cur = new_res.end + 1;
                        }
                    }

                    i += 1;
                }

                match max {
                    Some(max) => {
                        // If min == max, we're done; move on!
                        if *min == *max {
                            return self.exec(res, node.next.as_ref(), cur);
                        }

                        // Branch two states: one where we don't match again, and one where we match {1, max-min}

                        // Push the "don't match" state.
                        self.frontier.push_front(ExecutorState {
                            res: res.clone(),
                            node: node.next.clone(),
                            cur,
                        });

                        // Push the "match again {1,max-min}" state.
                        self.frontier.push_front(ExecutorState {
                            res,
                            node: Some(
                                MutNode {
                                    val: NodeVal::RepetitionRange {
                                        min: 1,
                                        max: Some(*max - *min),
                                        node: to_test.clone(),
                                    },
                                    next: node
                                        .next
                                        .clone()
                                        .map(|node| Rc::new(RefCell::new(node.as_ref().clone().into_mut()))),
                                }
                                .into_node()
                                .into(),
                            ),
                            cur,
                        });

                        return Ok(None);
                    }
                    None => {
                        // We don't have an upper limit; push a "zero-or-more" state.
                        self.frontier.push_front(ExecutorState {
                            res,
                            node: Some(Rc::new(Node {
                                val: NodeVal::ZeroOrMore {
                                    node: to_test.clone(),
                                    greedy: false,
                                },
                                next: node.next.clone(),
                            })),
                            cur,
                        });

                        return Ok(None);
                    }
                }
            }
            NodeVal::Group { group, cfg: group_cfg } => {
                // Take the inner group and append a GroupEnd val that will mark the end of the group when we hit it
                // (which means we don't have to deal with nested states, especially when exploring different expression branches in the frontier).

                let new_head = group.as_ref().clone().into_mut().append(MutNode {
                    val: NodeVal::GroupEnd {
                        start: cur,
                        cfg: group_cfg.clone(),
                    },
                    next: match &node.next {
                        None => None,
                        Some(next) => Some(Rc::new(RefCell::new(next.as_ref().clone().into()))),
                    },
                });

                self.exec(res, Some(&Rc::new(new_head.into_node())), cur)
            }
            NodeVal::GroupEnd { start, cfg } => {
                // Record this group if needed.
                let mut res = res;
                if let (Some(res), Some(group_cfg)) = (&mut res, cfg) {
                    match group_cfg {
                        parser::GroupConfig::NonCapturing => {}
                        parser::GroupConfig::Named(name) => {
                            res.groups.insert(name.clone(), (*start, cur - 1));
                        }
                    }
                }

                self.exec(res, node.next.as_ref(), cur)
            }
            NodeVal::Set { set, inverted } => {
                let ch = match self.input.chars().nth(cur) {
                    None => return Ok(None),
                    Some(ch) => ch,
                };

                match (inverted, set.contains(&ch)) {
                    // not inverted, did find:
                    (false, true) | (true, false) => {
                        self.exec(res.or(Some(ExecResult::new(cur))), node.next.as_ref(), cur + 1)
                    }
                    _ => Ok(None),
                }
            }
            NodeVal::Or { left, right } => {
                // Emit two states: one where we take the left and one where we take the right.
                // NOTE: this might require more plumbing because we might want to allow either as a valid match; not sure what the actual spec is here.

                // Push the right state.
                self.frontier.push_front(ExecutorState {
                    res: res.clone(),
                    node: Some(
                        left.as_ref()
                            .clone()
                            .into_mut()
                            .append_option_node(&node.next)
                            .into_node()
                            .into(),
                    ),
                    cur,
                });

                self.frontier.push_front(ExecutorState {
                    res: res.clone(),
                    node: Some(
                        right
                            .as_ref()
                            .clone()
                            .into_mut()
                            .append_option_node(&node.next)
                            .into_node()
                            .into(),
                    ),
                    cur,
                });

                return Ok(None);
            }
        }
    }
}
