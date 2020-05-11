use std::rc::Rc;
use std::cell::RefCell;

use crate::treenodes::TreeNode;

pub type WrappedNode = Rc<RefCell<Node>>;

#[derive(Debug)]
pub struct Node {
    left: Option<WrappedNode>,
    right: Option<WrappedNode>,
}

impl Node {
    pub fn new() -> Rc<RefCell<Self>> {
        let node = Node {
            left: None,
            right: None,
        };

        Rc::new(RefCell::new(node))
    }
}

impl TreeNode for Node {
    type EdgeRepr = WrappedNode;

    fn left(&self) -> Option<Self::EdgeRepr> {
        self.left.as_ref().map(Rc::clone)
    }

    fn right(&self) -> Option<Self::EdgeRepr> {
        self.right.as_ref().map(Rc::clone)
    }

    fn set_edges(&mut self, l: Self::EdgeRepr, r: Self::EdgeRepr) {
        self.left = Some(l);
        self.right = Some(r);
    }
}
