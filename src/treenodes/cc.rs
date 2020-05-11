use bacon_rajan_cc::{Cc, Trace, Tracer, Weak};
use std::cell::RefCell;

use crate::treenodes::TreeNode;

pub type WrappedNode = Cc<RefCell<CcNode>>;

#[derive(Debug)]
pub struct CcNode {
    left: Option<WrappedNode>,
    right: Option<WrappedNode>,
}

impl CcNode {
    pub fn new() -> Cc<RefCell<Self>> {
        let node = CcNode {
            left: None,
            right: None,
        };

        Cc::new(RefCell::new(node))
    }
}

impl Trace for CcNode {
    fn trace(&mut self, tracer: &mut Tracer) {}
}

impl TreeNode for CcNode {
    type EdgeRepr = WrappedNode;

    fn left(&self) -> Option<Self::EdgeRepr> {
        self.left.as_ref().map(Cc::clone)
    }

    fn right(&self) -> Option<Self::EdgeRepr> {
        self.right.as_ref().map(Cc::clone)
    }

    fn set_edges(&mut self, l: Self::EdgeRepr, r: Self::EdgeRepr) {
        self.left = Some(l);
        self.right = Some(r);
    }
}
