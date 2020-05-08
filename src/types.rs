pub trait AsNode {
    fn as_node(&self) -> NodeId;
}

pub trait WeightedEdge {
    fn get_weight(&self) -> usize;
    fn set_weight(&mut self, weight: usize);
}

pub type NodeId = usize;
pub type DestId = NodeId;
pub type Weight = NodeId;

pub type Edge = (NodeId, DestId, Option<Weight>);
pub type WEdge = (NodeId, DestId, Weight);
pub type EdgeList = Vec<Edge>;
pub type WEdgeList = Vec<WEdge>;

pub const NUM_TRIALS: usize = 1;
pub const SYMMETRIZE: bool = true;
pub const UNIFORM: bool = true;
pub const NEEDS_WEIGHTS: bool = true;
pub const FILE_NAME: &'static str = ""; // "datasets/dolphins.out"
pub const INVERT: bool = false;
pub const SCALE: usize = 12;
pub const DEGREE: usize = 20;
