pub type NodeId = usize;
pub type DestId = NodeId;
pub type Weight = NodeId;

pub type Edge = (NodeId, DestId);
pub type WEdge = (NodeId, DestId, Weight);
pub type EdgeList = Vec<Edge>;