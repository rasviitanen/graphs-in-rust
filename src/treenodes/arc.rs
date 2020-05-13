use crate::treenodes::TreeNode;
use std::sync::{Arc, RwLock};

pub type WrappedNode = Arc<RwLock<Node>>;

#[derive(Debug)]
pub struct Node {
    left: Option<WrappedNode>,
    right: Option<WrappedNode>,
}

impl Node {
    pub fn new() -> WrappedNode {
        let node = Node {
            left: None,
            right: None,
        };

        Arc::new(RwLock::new(node))
    }
}

impl TreeNode for Node {
    type EdgeRepr = WrappedNode;

    fn left(&self) -> Option<Self::EdgeRepr> {
        self.left.as_ref().map(Arc::clone)
    }

    fn right(&self) -> Option<Self::EdgeRepr> {
        self.right.as_ref().map(Arc::clone)
    }

    fn set_edges(&mut self, l: Self::EdgeRepr, r: Self::EdgeRepr) {
        self.left = Some(l);
        self.right = Some(r);
    }
}
