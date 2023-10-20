use std::{rc::Rc, cell::RefCell};

use crate::parser;

use super::ExecError;

pub enum ExecNodeVal {
    // Poisoned is a special value that represents a NodeVal that has been poisoned.
    Poisoned,

    Word(String),
    Any,
    ZeroOrMore(Box<ExecNode>),
    OneOrMore(Box<ExecNode>),
    Start,
    End,
    Optional(Box<ExecNode>),
    Group {
        group: Box<ExecNode>,
        cfg: Option<parser::GroupConfig>,
    },
    Set {
        set: indexmap::IndexSet<char>,
        inverted: bool,
    },
    Or {
        left: Box<ExecNode>,
        right: Box<ExecNode>,
    },
    RepetitionRange {
        min: u32,
        max: Option<u32>,
        node: Box<ExecNode>,
    },
}

pub struct ExecNode {
    pub val: ExecNodeVal,
    pub next: Option<Box<ExecNode>>,
}

impl ExecNode {
    pub fn from_parsed(parsed_node: Rc<RefCell<parser::Node>>) -> Result<Box<Self>, ExecError> {
        let parsed_node = Rc::try_unwrap(parsed_node)
            .map_err(|_| ExecError::ParseGraphCycle)?
            .into_inner();

        let val = match parsed_node.val {
            parser::NodeVal::Poisoned => ExecNodeVal::Poisoned,
            parser::NodeVal::Word(word) => ExecNodeVal::Word(word),
            parser::NodeVal::Any => ExecNodeVal::Any,
            parser::NodeVal::ZeroOrMore(node) => {
                ExecNodeVal::ZeroOrMore(Self::from_parsed(node)?)
            }
            parser::NodeVal::OneOrMore(node) => {
                ExecNodeVal::OneOrMore(Self::from_parsed(node)?)
            }
            parser::NodeVal::Start => ExecNodeVal::Start,
            parser::NodeVal::End => ExecNodeVal::End,
            parser::NodeVal::Optional(node) => {
                ExecNodeVal::Optional(Self::from_parsed(node)?)
            }
            parser::NodeVal::Group { group, cfg } => ExecNodeVal::Group {
                group: Self::from_parsed(group)?,
                cfg,
            },
            parser::NodeVal::Set { set, inverted } => ExecNodeVal::Set { set, inverted },
            parser::NodeVal::Or { left, right } => ExecNodeVal::Or {
                left: Self::from_parsed(left)?,
                right: Self::from_parsed(right)?,
            },
            parser::NodeVal::RepetitionRange { min, max, node } => {
                ExecNodeVal::RepetitionRange {
                    min,
                    max,
                    node: Self::from_parsed(node)?,
                }
            }
        };

        Ok(Box::new(ExecNode {
            val,
            next: match parsed_node.next {
                None => None,
                Some(next) => Some(Self::from_parsed(next)?),
            },
        }))
    }
}
