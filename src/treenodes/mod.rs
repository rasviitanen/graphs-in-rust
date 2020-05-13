pub trait TreeNode {
    type EdgeRepr;

    fn left(&self) -> Option<Self::EdgeRepr>;
    fn right(&self) -> Option<Self::EdgeRepr>;
    fn set_edges(&mut self, l: Self::EdgeRepr, r: Self::EdgeRepr);
}

pub mod arc;
pub mod cc;
pub mod msgc;
pub mod rc;
