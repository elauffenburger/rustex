use std::{cell::RefCell, rc::Rc};

use indexmap::IndexSet;

pub struct ParseNode {
    pub val: ParseNodeVal,
    pub next: Option<Rc<RefCell<ParseNode>>>,
}

#[derive(Clone)]
pub enum ParseNodeVal {
    // Poisoned is a special value that represents a NodeVal that has been poisoned.
    Poisoned,

    Word(String),
    Any,
    ZeroOrMore {
        node: Rc<RefCell<ParseNode>>,
        greedy: bool,
    },
    OneOrMore {
        node: Rc<RefCell<ParseNode>>,
        greedy: bool,
    },
    Start,
    End,
    Optional(Rc<RefCell<ParseNode>>),
    Group {
        group: Rc<RefCell<ParseNode>>,
        cfg: Option<super::GroupConfig>,
    },
    Set {
        set: IndexSet<char>,
        inverted: bool,
    },
    Or {
        left: Rc<RefCell<ParseNode>>,
        right: Rc<RefCell<ParseNode>>,
    },
    RepetitionRange {
        min: u32,
        max: Option<u32>,
        node: Rc<RefCell<ParseNode>>,
    },
}
