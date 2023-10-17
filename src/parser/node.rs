use std::{cell::RefCell, fmt, rc::Rc, collections};

#[derive(Debug, Clone)]
pub enum GroupConfig {
    NonCapturing,
    Named(String),
}

pub struct Node {
    pub val: NodeVal,
    pub next: Option<Rc<RefCell<Node>>>,
}

impl Node {
    fn fmt_internal(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.val {
            NodeVal::Poisoned => f.write_str("!!poison!!"),
            NodeVal::Word(word) => f.write_fmt(format_args!("'{}'", word)),
            NodeVal::Any => f.write_str("."),
            NodeVal::ZeroOrMore(node) => {
                node.borrow().fmt_internal(f)?;
                f.write_str("*")
            }
            NodeVal::OneOrMore(node) => {
                node.borrow().fmt_internal(f)?;
                f.write_str("+")
            }
            NodeVal::Start => f.write_str("^"),
            NodeVal::End => f.write_str("$"),
            NodeVal::Optional(node) => {
                node.borrow().fmt_internal(f)?;
                f.write_str("?")
            }
            NodeVal::Group { group, cfg } => {
                f.write_str("(")?;

                match cfg {
                    None => {}
                    Some(GroupConfig::Named(name)) => {
                        f.write_fmt(format_args!("<{}>", name))?;
                    }
                    Some(GroupConfig::NonCapturing) => {
                        f.write_str("?:")?;
                    }
                }

                group.as_ref().borrow().fmt_internal(f)?;

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
                left.as_ref().borrow().fmt_internal(f)?;
                f.write_str(">|<")?;
                right.as_ref().borrow().fmt_internal(f)?;
                f.write_str(">")
            }
            NodeVal::RepetitionRange { min, max, node } => {
                node.borrow().fmt_internal(f)?;

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
                node.as_ref().borrow().fmt_internal(f)
            }
        }
    }
}

impl fmt::Debug for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_internal(f)
    }
}

#[derive(Debug, Clone)]
pub enum NodeVal {
    // Poisoned is a special value that represents a NodeVal that has been poisoned.
    Poisoned,

    Word(String),
    Any,
    ZeroOrMore(Rc<RefCell<Node>>),
    OneOrMore(Rc<RefCell<Node>>),
    Start,
    End,
    Optional(Rc<RefCell<Node>>),
    Group {
        group: Rc<RefCell<Node>>,
        cfg: Option<GroupConfig>,
    },
    Set {
        set: collections::HashSet<char>,
        inverted: bool,
    },
    Or {
        left: Rc<RefCell<Node>>,
        right: Rc<RefCell<Node>>,
    },
    RepetitionRange {
        min: u32,
        max: Option<u32>,
        node: Rc<RefCell<Node>>,
    },
}
