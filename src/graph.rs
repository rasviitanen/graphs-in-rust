use crate::types::*;

pub type Range<'a, T> = Box<dyn Iterator<Item = T> + 'a>;

pub trait CSRGraph<V, E> {
    fn op_add_vertex(&self, v: NodeId);
    fn op_add_edge(&self, v: NodeId, e: NodeId);
    fn op_delete_edge(&self, v: NodeId, e: NodeId);
    fn op_delete_vertex(&self, v: NodeId);
    fn op_find_vertex(&self, v: NodeId);

    fn build_directed(num_nodes: usize, edge_list: &EdgeList) -> Self;
    fn build_undirected(num_nodes: usize, edge_list: &EdgeList) -> Self;

    fn directed(&self) -> bool;

    fn num_nodes(&self) -> usize;
    fn num_edges(&self) -> usize;
    fn num_edges_directed(&self) -> usize;

    fn out_degree(&self, v: NodeId) -> usize;
    fn in_degree(&self, v: NodeId) -> usize;

    fn in_neigh(&self, v: NodeId) -> Range<E>;
    fn out_neigh(&self, v: NodeId) -> Range<E>;

    fn print_stats(&self);

    fn vertices(&self) -> Range<V>;

    fn old_bfs(&self, v: NodeId);

    fn replace_out_edges(&self, v: NodeId, edges: Vec<E>);
    fn replace_in_edges(&self, v: NodeId, edges: Vec<E>);
}
