use std::{cell::RefCell, rc::Rc, sync::Arc};

use crate::parser::{Node, NodeVal};

pub trait AppendNode: Sized {
    fn with_tail(&self, tail: Self) -> Self;
    fn with_tail_option(&self, tail: Option<Self>) -> Self;
}

impl AppendNode for Arc<Node> {
    fn with_tail_option(&self, tail: Option<Self>) -> Self {
        self.as_ref()
            .clone()
            .into_mut()
            .append_option_node(&tail)
            .into_node()
            .arc()
    }

    fn with_tail(&self, tail: Self) -> Self {
        self.with_tail_option(Some(tail))
    }
}

pub trait IntoMutNode {
    fn into_mut(self) -> MutNode;
}

impl IntoMutNode for Node {
    fn into_mut(self) -> MutNode {
        self.into()
    }
}

#[derive(Clone, Debug)]
pub struct MutNode {
    pub val: NodeVal,
    pub next: Option<Rc<RefCell<MutNode>>>,
}

impl MutNode {
    pub fn tail(head: Rc<RefCell<Self>>) -> Rc<RefCell<MutNode>> {
        let mut curr = head;
        while let Some(next) = &curr.clone().borrow().next {
            curr = next.clone();
        }

        curr
    }

    pub fn append(self, node: MutNode) -> Self {
        // We need to perform all operations on head_mut inside a separate scope so the bindings will be dropped before the Arc::try_unwrap.
        let head_mut = Rc::new(RefCell::new(self));
        {
            let tail_mut = MutNode::tail(head_mut.clone());

            // Add a GroupEnd to the end of the group and then add the rest of the node chain after that.
            tail_mut.borrow_mut().next.replace(Rc::new(RefCell::new(node)));
        }

        Rc::try_unwrap(head_mut)
            .expect("MutNode should not have created multiple Arc refs to inner nodes")
            .into_inner()
    }

    pub fn append_option_node(self, next: &Option<Arc<Node>>) -> Self {
        match next {
            None => self,
            Some(next) => self.append(next.as_ref().clone().into_mut()),
        }
    }

    pub fn into_node(self) -> Node {
        self.try_into()
            .expect("MutNode should not have created multiple Arc refs to inner nodes")
    }
}

impl From<Node> for MutNode {
    fn from(node: Node) -> Self {
        fn rec(maybe_node: &Option<Arc<Node>>) -> Option<Rc<RefCell<MutNode>>> {
            maybe_node.as_ref().map(|node| {
                Rc::new(RefCell::new(MutNode {
                    val: node.val.clone(),
                    next: rec(&node.next),
                }))
            })
        }

        MutNode {
            val: node.val.clone(),
            next: rec(&node.next),
        }
    }
}

impl TryFrom<MutNode> for Node {
    type Error = Rc<RefCell<MutNode>>;

    fn try_from(node: MutNode) -> Result<Self, Self::Error> {
        fn try_unwrap_mut_node(mut_node: Rc<RefCell<MutNode>>) -> Result<MutNode, Rc<RefCell<MutNode>>> {
            Rc::try_unwrap(mut_node).map(|refcell| refcell.into_inner())
        }

        Ok(Node {
            val: node.val.clone(),
            next: match node.next {
                None => None,
                Some(node) => Some(Arc::new(try_unwrap_mut_node(node)?.try_into()?)),
            },
        })
    }
}

pub trait IntoSmartPointer: Sized {
    fn arc(self) -> Arc<Self> {
        Arc::new(self)
    }
}

impl<A> IntoSmartPointer for A {}
