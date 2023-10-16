use std::{cell::RefCell, fmt, rc::Rc};

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
            NodeVal::Word(word) => f.write_fmt(format_args!("'{}'", word)),
            NodeVal::Any => f.write_str("."),
            NodeVal::ZeroOrMore => f.write_str("*"),
            NodeVal::OneOrMore => f.write_str("+"),
            NodeVal::Start => f.write_str("^"),
            NodeVal::End => f.write_str("$"),
            NodeVal::Optional => f.write_str("?"),
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
    Word(String),
    Any,
    ZeroOrMore,
    OneOrMore,
    Start,
    End,
    Optional,
    Group {
        group: Rc<RefCell<Node>>,
        cfg: Option<GroupConfig>,
    },
    Set {
        set: Vec<char>,
        inverted: bool,
    },
    Or {
        left: Rc<RefCell<Node>>,
        right: Rc<RefCell<Node>>,
    },
}
