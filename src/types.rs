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

// GAP BENCHMARK SUITE
pub const NUM_TRIALS: usize = 1;
pub const SYMMETRIZE: bool = true; // false = undirected
pub const UNIFORM: bool = true;
pub const NEEDS_WEIGHTS: bool = true;
pub const FILE_NAME: &'static str = ""; // ""
pub const INVERT: bool = false;
pub const SCALE: usize = 8;
pub const DEGREE: usize = 10;

// GC BENCH
pub const GRAPH_SIZE: i64 = 1 << 18;
pub const kStretchTreeDepth: i32 = 16; // 18;
pub const kLongLivedTreeDepth: i32 = 14; // 16;
pub const kMaxTreeDepth: i32 = 14; // 16;
pub const kArraySize: i32 = 500000;
pub const kMinTreeDepth: i32 = 4;
