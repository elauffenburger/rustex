use std::{cell::RefCell, fmt, sync::Arc};

use indexmap::IndexSet;

use crate::parser::ParseNodeVal;

use super::ParseNode;

#[derive(Clone)]
pub struct Node {
    pub val: NodeVal,
    pub next: Option<Arc<Node>>,
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

                        if iter.peek().is_some() {
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
            NodeVal::GroupEnd { .. } => {
                f.write_str("(/)")?;

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

impl TryFrom<ParseNode> for Node {
    type Error = super::ParseError;

    fn try_from(parsed_node: ParseNode) -> Result<Self, Self::Error> {
        fn try_unwrap_parse_node(parse_node: Arc<RefCell<ParseNode>>) -> Result<ParseNode, super::ParseError> {
            Arc::try_unwrap(parse_node)
                .map_err(|_| super::ParseError::ParseGraphCycle)
                .map(|refcell| refcell.into_inner())
        }

        let val = match parsed_node.val {
            ParseNodeVal::Poisoned => NodeVal::Poisoned,
            ParseNodeVal::Word(word) => NodeVal::Word(word),
            ParseNodeVal::Any => NodeVal::Any,
            ParseNodeVal::ZeroOrMore { node, greedy } => NodeVal::ZeroOrMore {
                node: Arc::new(try_unwrap_parse_node(node)?.try_into()?),
                greedy,
            },
            ParseNodeVal::OneOrMore { node, greedy } => NodeVal::OneOrMore {
                node: Arc::new(try_unwrap_parse_node(node)?.try_into()?),
                greedy,
            },
            ParseNodeVal::Start => NodeVal::Start,
            ParseNodeVal::End => NodeVal::End,
            ParseNodeVal::Optional(node) => NodeVal::Optional(Arc::new(try_unwrap_parse_node(node)?.try_into()?)),
            ParseNodeVal::Group { group, cfg } => NodeVal::Group {
                group: Arc::new(try_unwrap_parse_node(group)?.try_into()?),
                cfg,
            },
            ParseNodeVal::Set { set, inverted } => NodeVal::Set { set, inverted },
            ParseNodeVal::Or { left, right } => NodeVal::Or {
                left: Arc::new(try_unwrap_parse_node(left)?.try_into()?),
                right: Arc::new(try_unwrap_parse_node(right)?.try_into()?),
            },
            ParseNodeVal::RepetitionRange { min, max, node } => NodeVal::RepetitionRange {
                min,
                max,
                node: Arc::new(try_unwrap_parse_node(node)?.try_into()?),
            },
        };

        Ok(Node {
            val,
            next: match parsed_node.next {
                None => None,
                Some(next) => Some(Arc::new(try_unwrap_parse_node(next)?.try_into()?)),
            },
        })
    }
}

#[derive(Debug, Clone)]
pub enum NodeVal {
    // Poisoned is a special value that represents a NodeVal that has been poisoned.
    Poisoned,

    Word(String),
    Any,
    ZeroOrMore {
        node: Arc<Node>,
        greedy: bool,
    },
    OneOrMore {
        node: Arc<Node>,
        greedy: bool,
    },
    Start,
    End,
    Optional(Arc<Node>),
    Group {
        group: Arc<Node>,
        cfg: Option<super::GroupConfig>,
    },
    GroupEnd {
        start: usize,
        cfg: Option<super::GroupConfig>,
    },
    Set {
        set: IndexSet<char>,
        inverted: bool,
    },
    Or {
        left: Arc<Node>,
        right: Arc<Node>,
    },
    RepetitionRange {
        min: u32,
        max: Option<u32>,
        node: Arc<Node>,
    },
}
