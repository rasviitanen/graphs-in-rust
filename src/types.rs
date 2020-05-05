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