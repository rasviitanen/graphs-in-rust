use crate::types::*;

pub type Range<'a, T> = Box<dyn Iterator<Item = T> + 'a>;

pub trait CSRGraph {
    fn directed(&self) -> bool;

    fn num_nodes(&self) -> usize;
    fn num_edges(&self) -> usize;
    fn num_edges_directed(&self) -> usize;

    fn out_degree(&self, v: NodeId) -> usize;
    fn in_degree(&self, v: NodeId) -> usize;


    fn print_stats(&self);

    fn vertices<T>(&self) -> Range<T>;
}