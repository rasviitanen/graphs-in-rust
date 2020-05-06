// use crate::types::*;
// use fastgraph::bloom;
// use std::cell::{RefCell, Cell};
// use std::collections::{HashMap, HashSet, VecDeque};
// use crate::graph::{Range, CSRGraph};
// use epoch::{Atomic, Guard, Owned, Shared};
// use fastgraph::bloom::Edge;

// #[derive(Clone)]
// pub struct WeightedEdgeInfo<'a> {
//     inner: bloom::EdgeInfo<'a, usize>,
//     weight: Option<usize>,
// }

// impl<'a> WeightedEdge for WeightedEdgeInfo<'a> {
//     fn get_weight(&self) -> usize {
//         self.weight.expect("Weights must be assigned before used")
//     }

//     fn set_weight(&mut self, weight: usize) {
//         self.weight.replace(weight);
//     }
// }

// impl<'a> AsNode for WeightedEdgeInfo<'a> {
//     fn as_node(&self) -> NodeId {
//         self.inner.key()
//     }
// }

// impl<'a> std::ops::Deref for WeightedEdgeInfo<'a> {
//     type Target = bloom::EdgeInfo<'a, usize>;

//     fn deref(&self) -> &Self::Target {
//         &self.inner
//     }
// }

// pub struct Graph<'a> {
//     inner: bloom::Graph::<'a, usize, bloom::EdgeInfo<'a, usize>>,
//     n_edges: Cell<usize>,
//     directed: bool,
// }

// type Vertex<'a> = bloom::RefEntry<'a, usize, bloom::EdgeInfo<'a, usize>>;

// impl<'a> CSRGraph<Vertex<'a>, WeightedEdgeInfo<'a>> for Graph<'a> {
//     fn build_directed(num_nodes: usize, edge_list: &EdgeList) -> Self {
//         let graph = Self {
//             inner: bloom::Graph::new(num_nodes as i64),
//             n_edges: Cell::new(0),
//             directed: true,
//         };

//         for v in 0..num_nodes {
//             graph.inner.add_vertex(v, None);
//         }

//         for (v, e, w) in edge_list {
//             // graph.inner.add_edge(*v, *e, w, true)
//             unimplemented!();
//         }
//         graph
//     }

//     fn build_undirected(num_nodes: usize, edge_list: &EdgeList) -> Self {
//         let graph =  Self {
//             inner: bloom::Graph::new(num_nodes as i64),
//             n_edges: Cell::new(0),
//             directed: false,
//         };

//         for v in 0..num_nodes {
//             graph.inner.add_vertex(v, None);
//         }
//         for (v, e, w) in edge_list {
//             // graph.inner.add_edge(*v, *e, w, false);
//             unimplemented!();
//         }

//         graph
//     }

//     fn directed(&self) -> bool {
//         self.directed
//     }

//     fn num_nodes(&self) -> usize {
//         // self.inner.len()
//         unimplemented!();
//     }

//     fn num_edges(&self) -> usize {
//         self.n_edges.get()
//     }

//     fn num_edges_directed(&self) -> usize {
//         let mut sum = 0;
//         for v in self.vertices() {
//             // sum += v.out_edges.len();
//             unimplemented!();
//         }
//         sum
//     }

//     fn out_degree(&self, v: NodeId) -> usize {
//         if let Some(found) = self.inner.find_vertex(v) {
//             // found.out_edges.len()
//             unimplemented!();
//         } else {
//             0
//         }
//     }

//     fn in_degree(&self, v: NodeId) -> usize {
//         println!("Graph inversion is probably disabled... in in_degree()");
//         if let Some(found) = self.inner.find_vertex(v) {
//             // found.in_edges.len()
//             unimplemented!();
//         } else {
//             panic!("Vertex not found");
//         }
//     }

//     fn out_neigh(&self, v: NodeId) -> Range<WeightedEdgeInfo<'a>> {
//         // if let Some(vertex) = self.vertices.borrow().get(&v) {
//         //     let mut edges = Vec::new();
//         //     for edge in vertex.borrow().out_edges.values() {
//         //         edges.push(WrappedNode::clone(&edge));
//         //     }
//         //     edges.sort_by(|a, b| a.as_node().cmp(&b.as_node()));
//         //     Box::new(edges.into_iter())
//         // } else {
//         //     panic!("Vertex not found");
//         // }
//         unimplemented!();
//     }

//     fn in_neigh(&self, v: NodeId) -> Range<WeightedEdgeInfo<'a>> {
//         // if let Some(vertex) = self.vertices.borrow().get(&v) {
//         //     let mut edges = Vec::new();
//         //     for edge in vertex.borrow().in_edges.values() {
//         //         edges.push(WrappedNode::clone(&edge));
//         //     }
//         //     edges.sort_by(|a, b| a.as_node().cmp(&b.as_node()));
//         //     Box::new(edges.into_iter())
//         // } else {
//         //     panic!("Vertex not found");
//         // }
//         unimplemented!();
//     }

//     fn print_stats(&self) {
//         println!("---------- GRAPH ----------");
//         println!("  Num Nodes          - {:?}", self.num_nodes());
//         println!("  Num Edges          - {:?}", self.num_edges_directed());
//         println!("---------------------------");
//     }

//     fn vertices(&self) -> Range<Vertex<'a>> {
//         let mut edges = Vec::new();
//         let guard = &epoch::pin();
//         for edge in self.inner.vertices(guard) {
//             edges.push(Vertex::clone(&edge));
//         }
//         edges.sort_by(|a, b| a.get().key.cmp(&b.get().key));
//         Box::new(edges.into_iter())
//     }

//     fn replace_out_edges(&self, v: NodeId, edges: Vec<WeightedEdgeInfo<'a>>) {
//         unimplemented!();
//         // if let Some(vertex) = self.inner.find_vertex(v) {
//         //     let mut new_edges = HashMap::new();
//         //     for e in edges {
//         //         new_edges.insert(e.as_node(), e);
//         //     }
//         //     vertex.out_edges = new_edges;
//         // }
//     }

//     fn replace_in_edges(&self, v: NodeId, edges: Vec<WeightedEdgeInfo<'a>>) {
//         unimplemented!();
//         // if let Some(vertex) = self.inner.find_vertex(v) {
//         //     let mut new_edges = HashMap::new();
//         //     for e in edges {
//         //         new_edges.insert(e.as_node(), e);
//         //     }
//         //     vertex.in_edges = new_edges;
//         // }
//     }

//     fn old_bfs(&self, v: NodeId) {
//         unimplemented!();
//         // self.bfs(v, None);
//     }
// }