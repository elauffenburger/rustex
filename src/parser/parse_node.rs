use std::{cell::RefCell, sync::Arc};

use indexmap::IndexSet;

pub struct ParseNode {
    pub val: ParseNodeVal,
    pub next: Option<Arc<RefCell<ParseNode>>>,
}

#[derive(Clone)]
pub enum ParseNodeVal {
    // Poisoned is a special value that represents a NodeVal that has been poisoned.
    Poisoned,

    Word(String),
    Any,
    ZeroOrMore {
        node: Arc<RefCell<ParseNode>>,
        greedy: bool,
    },
    OneOrMore {
        node: Arc<RefCell<ParseNode>>,
        greedy: bool,
    },
    Start,
    End,
    Optional(Arc<RefCell<ParseNode>>),
    Group {
        group: Arc<RefCell<ParseNode>>,
        cfg: Option<super::GroupConfig>,
    },
    Set {
        set: IndexSet<char>,
        inverted: bool,
    },
    Or {
        left: Arc<RefCell<ParseNode>>,
        right: Arc<RefCell<ParseNode>>,
    },
    RepetitionRange {
        min: u32,
        max: Option<u32>,
        node: Arc<RefCell<ParseNode>>,
    },
}
