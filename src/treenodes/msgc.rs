use gc::{Finalize, Gc, GcCell, Trace};
use std::cell::RefCell;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet, VecDeque};
use crate::treenodes::TreeNode;

pub type WrappedNode = Gc<GcCell<Node>>;

#[derive(Trace, Finalize)]
pub struct Node {
    left:  Option<Gc<GcCell<Self>>>,
    right: Option<Gc<GcCell<Self>>>,
}

impl Node {
    pub fn new() -> Gc<GcCell<Self>> {
        let node = Node {
            left: None,
            right: None,
        };

        Gc::new(GcCell::new(node))
    }
}

impl TreeNode for Node {
    type EdgeRepr = WrappedNode;

    fn left(&self) -> Option<Self::EdgeRepr> {
        self.left.as_ref().map(Gc::clone)
    }

    fn right(&self) -> Option<Self::EdgeRepr> {
        self.right.as_ref().map(Gc::clone)
    }

    fn set_edges(&mut self, l: Self::EdgeRepr, r: Self::EdgeRepr) {
        self.left = Some(l);
        self.right = Some(r);
    }
}
