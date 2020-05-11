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


// HAP BENCHMARK SOUT
pub const NUM_TRIALS: usize = 1;
pub const SYMMETRIZE: bool = false;
pub const UNIFORM: bool = false;
pub const NEEDS_WEIGHTS: bool = true;
pub const FILE_NAME: &'static str = ""; // "datasets/dolphins.out"
pub const INVERT: bool = false;
pub const SCALE: usize = 10;
pub const DEGREE: usize = 4;


// GC BENCH
pub const GRAPH_SIZE: i64 = 1 << 18;
pub const kStretchTreeDepth: i32 =   18; // 18;
pub const kLongLivedTreeDepth: i32 = 16; // 16;
pub const kMaxTreeDepth: i32 =       16; // 16;
pub const kArraySize: i32 = 500000;
pub const kMinTreeDepth: i32 = 4;