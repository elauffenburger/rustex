use std::{cell::RefCell, fmt, rc::Rc};

use indexmap::IndexSet;

use super::parse_node::*;

pub struct Node {
    pub val: NodeVal,
    pub next: Option<Box<Node>>,
}

impl Clone for Node {
    fn clone(&self) -> Self {
        Self {
            val: self.val.clone(),
            next: self.next.clone(),
        }
    }
}

impl Node {
    pub fn from_parsed(
        parsed_node: Rc<RefCell<ParseNode>>,
    ) -> Result<Box<Self>, super::ParseError> {
        let parsed_node = Rc::try_unwrap(parsed_node)
            .map_err(|_| super::ParseError::ParseGraphCycle)?
            .into_inner();

        let val = match parsed_node.val {
            ParseNodeVal::Poisoned => NodeVal::Poisoned,
            ParseNodeVal::Word(word) => NodeVal::Word(word),
            ParseNodeVal::Any => NodeVal::Any,
            ParseNodeVal::ZeroOrMore { node, greedy } => NodeVal::ZeroOrMore {
                node: Self::from_parsed(node)?,
                greedy,
            },
            ParseNodeVal::OneOrMore { node, greedy } => NodeVal::OneOrMore {
                node: Self::from_parsed(node)?,
                greedy,
            },
            ParseNodeVal::Start => NodeVal::Start,
            ParseNodeVal::End => NodeVal::End,
            ParseNodeVal::Optional(node) => NodeVal::Optional(Self::from_parsed(node)?),
            ParseNodeVal::Group { group, cfg } => NodeVal::Group {
                group: Self::from_parsed(group)?,
                cfg,
            },
            ParseNodeVal::Set { set, inverted } => NodeVal::Set { set, inverted },
            ParseNodeVal::Or { left, right } => NodeVal::Or {
                left: Self::from_parsed(left)?,
                right: Self::from_parsed(right)?,
            },
            ParseNodeVal::RepetitionRange { min, max, node } => NodeVal::RepetitionRange {
                min,
                max,
                node: Self::from_parsed(node)?,
            },
        };

        Ok(Box::new(Node {
            val,
            next: match parsed_node.next {
                None => None,
                Some(next) => Some(Self::from_parsed(next)?),
            },
        }))
    }
}

impl fmt::Debug for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.val {
            NodeVal::Poisoned => f.write_str("!!poison!!"),
            NodeVal::Word(word) => f.write_fmt(format_args!("'{}'", word)),
            NodeVal::Any => f.write_str("."),
            NodeVal::ZeroOrMore { node, greedy } => {
                node.fmt(f)?;
                f.write_str("*")?;
                if !greedy {
                    f.write_str("?")?;
                }

                Ok(())
            }
            NodeVal::OneOrMore { node, greedy } => {
                node.fmt(f)?;
                f.write_str("+")?;
                if !greedy {
                    f.write_str("?")?;
                }

                Ok(())
            }
            NodeVal::Start => f.write_str("^"),
            NodeVal::End => f.write_str("$"),
            NodeVal::Optional(node) => {
                node.fmt(f)?;
                f.write_str("?")
            }
            NodeVal::Group { group, cfg } => {
                f.write_str("(")?;

                match cfg {
                    None => {}
                    Some(super::GroupConfig::Named(name)) => {
                        f.write_fmt(format_args!("<{}>", name))?;
                    }
                    Some(super::GroupConfig::NonCapturing) => {
                        f.write_str("?:")?;
                    }
                }

                group.fmt(f)?;

                f.write_str(")")
            }
            NodeVal::Set { set, inverted } => {
                f.write_str("[")?;
                if *inverted {
                    f.write_str("^")?;
                }

                {
                    let mut iter = set.iter().peekable();
                    while let Some(ch) = iter.next() {
                        f.write_fmt(format_args!("'{}'", ch))?;

                        if let Some(_) = iter.peek() {
                            f.write_str(", ")?;
                        }
                    }
                }

                f.write_str("]")
            }
            NodeVal::Or { left, right } => {
                f.write_str("<")?;
                left.fmt(f)?;
                f.write_str(">|<")?;
                right.fmt(f)?;
                f.write_str(">")
            }
            NodeVal::RepetitionRange { min, max, node } => {
                node.fmt(f)?;

                f.write_str("{")?;
                f.write_fmt(format_args!("{}", min))?;

                if let Some(max) = max {
                    f.write_fmt(format_args!(",{}", max))?;
                }

                f.write_str("}")?;

                Ok(())
            }
        }?;

        match &self.next {
            None => Ok(()),
            Some(node) => {
                f.write_str("->")?;
                node.fmt(f)
            }
        }
    }
}

#[derive(Debug)]
pub enum NodeVal {
    // Poisoned is a special value that represents a NodeVal that has been poisoned.
    Poisoned,

    Word(String),
    Any,
    ZeroOrMore {
        node: Box<Node>,
        greedy: bool,
    },
    OneOrMore {
        node: Box<Node>,
        greedy: bool,
    },
    Start,
    End,
    Optional(Box<Node>),
    Group {
        group: Box<Node>,
        cfg: Option<super::GroupConfig>,
    },
    Set {
        set: IndexSet<char>,
        inverted: bool,
    },
    Or {
        left: Box<Node>,
        right: Box<Node>,
    },
    RepetitionRange {
        min: u32,
        max: Option<u32>,
        node: Box<Node>,
    },
}

impl Clone for NodeVal {
    fn clone(&self) -> Self {
        match self {
            Self::Poisoned => Self::Poisoned,
            Self::Word(word) => Self::Word(word.clone()),
            Self::Any => Self::Any,
            Self::ZeroOrMore { node, greedy } => Self::ZeroOrMore {
                node: node.clone(),
                greedy: *greedy,
            },
            Self::OneOrMore { node, greedy } => Self::OneOrMore {
                node: node.clone(),
                greedy: *greedy,
            },
            Self::Start => Self::Start,
            Self::End => Self::End,
            Self::Optional(node) => Self::Optional(node.clone()),
            Self::Group { group, cfg } => Self::Group {
                group: group.clone(),
                cfg: cfg.clone(),
            },
            Self::Set { set, inverted } => Self::Set {
                set: set.clone(),
                inverted: *inverted,
            },
            Self::Or { left, right } => Self::Or {
                left: left.clone(),
                right: right.clone(),
            },
            Self::RepetitionRange { min, max, node } => Self::RepetitionRange {
                min: *min,
                max: *max,
                node: node.clone(),
            },
        }
    }
}
