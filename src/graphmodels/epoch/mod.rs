mod adjlist;
mod lftt;
mod mdlist;

use crate::graphmodels::epoch::adjlist::AdjacencyList;
pub use crate::graphmodels::epoch::adjlist::{IterRefEntry, Node, RefEntry};

pub use crate::graphmodels::epoch::lftt::{OpType, ReturnCode};
use std::sync::atomic::Ordering::{Relaxed, SeqCst};

use epoch::{Atomic, Guard, Shared};
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::AtomicUsize;
use std::sync::RwLock;

use crate::graph::{CSRGraph, Range};
use crate::types::*;

#[derive(Copy, Clone)]
enum Direction {
    In,
    Out,
    Both,
}

pub trait Edge<'a, T> {
    fn key(&self) -> usize;
    fn as_node(&self) -> Shared<'a, Node<'a, T, Self>>
    where
        Self: std::marker::Sized;
}

pub trait Vertex {
    fn key(&self) -> usize;
}

unsafe impl Send for EdgeInfo {}
unsafe impl Sync for EdgeInfo {}

#[derive(Copy, Clone)]
pub struct EdgeInfo {
    // pub vertex_ref: RefEntry<'a, 'a, T, Self>,
    direction: Direction,
    node_id: NodeId,
    weight: Option<Weight>,
}

// impl<'a, T> EdgeInfo<'a, T> {
//     pub fn new(vertex_ref: RefEntry<'a, 'a, T, Self>) -> Self {
//         EdgeInfo {
//             vertex_ref,
//             weight: None,
//         }
//     }
// }

impl WeightedEdge for EdgeInfo {
    fn get_weight(&self) -> usize {
        self.weight.expect("Weights must be assigned before used")
    }

    fn set_weight(&mut self, weight: usize) {
        self.weight.replace(weight);
    }
}

impl AsNode for EdgeInfo {
    fn as_node(&self) -> NodeId {
        self.node_id
    }
}

// impl<'a, T> Edge<'a, T> for EdgeInfo {
//     #[inline]
//     fn key(&self) -> usize {
//         unsafe { self.vertex_ref.node.as_ref().unwrap().key }
//     }

//     #[inline]
//     fn as_node(&self) -> Shared<'a, Node<'a, T, Self>> {
//         self.vertex_ref.node
//     }
// }

pub struct Graph<'a, T: Clone> {
    inner: AdjacencyList<'a, T, EdgeInfo>,
    directed: bool,
    num_nodes: usize,
}

type E = EdgeInfo;
impl<'a, T: 'a + Clone> Graph<'a, T> {
    pub fn new(size_hint: i64, directed: bool) -> Self {
        Self {
            inner: AdjacencyList::new(size_hint),
            directed,
            num_nodes: 0,
        }
    }

    pub fn execute_ops<'t>(
        &'t self,
        ops: Vec<OpType<'a, T, E>>,
    ) -> std::sync::mpsc::Receiver<ReturnCode<RefEntry<'a, 't, T, E>>> {
        self.inner.txn(ops).execute()
    }

    pub fn add_vertex<'t>(
        &'t self,
        key: usize,
        value: Option<T>,
    ) -> Option<(usize, RefEntry<'a, 't, T, E>)> {
        let op = OpType::Insert(key, value);
        let insertion_txn = self.inner.txn(vec![op]).execute();

        if let Ok(ReturnCode::Inserted(entry)) = insertion_txn.recv() {
            Some((key, entry))
        } else {
            None
        }
    }

    pub fn add_edge(&self, parent_id: usize, edge_info: E) {
        let op = OpType::InsertEdge(parent_id, edge_info.node_id, Some(edge_info));
        let insert_edge_txn = self.inner.txn(vec![op]);
        insert_edge_txn.execute().recv().expect("Txn failed");
    }

    pub fn add_empty_edge(&self, parent: usize, child: usize) {
        let op = OpType::InsertEdge(parent, child, None);
        let insert_edge_txn = self.inner.txn(vec![op]);
        insert_edge_txn.execute().recv().expect("Txn failed");
    }

    pub fn connect<'t>(parent: &RefEntry<'a, 't, T, E>, child: E) {
        unsafe {
            AdjacencyList::connect(parent, child.node_id, child);
        }
    }

    pub fn vertices<'t, 'g>(&'a self, guard: &'g Guard) -> IterRefEntry<'a, 't, 'g, T, E> {
        self.inner.iter(guard)
    }

    pub fn find_vertex<'t>(&'t self, key: usize) -> Option<RefEntry<'a, 't, T, E>> {
        let op = OpType::Find(key);
        let find_txn = self.inner.txn(vec![op]);
        let res = find_txn.execute();

        if let ReturnCode::Found(entry) = res.recv().unwrap() {
            Some(entry)
        } else {
            None
        }
    }

    pub fn delete_vertex<'t>(&'t self, key: usize) -> Option<RefEntry<'a, 't, T, E>> {
        let op = OpType::Delete(key);
        let insertion_txn = self.inner.txn(vec![op]).execute();

        if let Ok(ReturnCode::Deleted(entry)) = insertion_txn.recv() {
            Some(entry)
        } else {
            None
        }
    }

    pub fn delete_edge<'t>(&'t self, parent: usize, edge: usize) -> Result<(), ()> {
        let op = OpType::DeleteEdge(parent, edge);
        let insertion_txn = self.inner.txn(vec![op]).execute();

        if let Ok(ReturnCode::Success) = insertion_txn.recv() {
            Ok(())
        } else {
            Err(())
        }
    }
}

impl<'a, V: Clone> CSRGraph<V, EdgeInfo> for Graph<'_, V> {
    fn build_directed(num_nodes: usize, edge_list: &EdgeList) -> Self {
        let mut graph = Graph::new(num_nodes as i64, true);

        for v in 0..num_nodes {
            graph.add_vertex(v, None);
        }

        graph.num_nodes = num_nodes;

        for (v, e, w) in edge_list {
            let edge_info_ev = EdgeInfo {
                direction: Direction::In,
                node_id: *v,
                weight: w.as_ref().map(|x| *x),
            };

            let edge_info_ve = EdgeInfo {
                direction: Direction::Out,
                node_id: *e,
                weight: w.as_ref().map(|x| *x),
            };

            graph.add_edge(*e, edge_info_ev);
            graph.add_edge(*v, edge_info_ve);
        }

        graph
    }

    fn build_undirected(num_nodes: usize, edge_list: &EdgeList) -> Self {
        let mut graph = Graph::new(num_nodes as i64, false);

        for v in 0..num_nodes {
            graph.add_vertex(v, None);
        }

        graph.num_nodes = num_nodes;

        for (v, e, w) in edge_list {
            let edge_info_ev = EdgeInfo {
                direction: Direction::Out,
                node_id: *v,
                weight: w.as_ref().map(|x| *x),
            };

            let edge_info_ve = EdgeInfo {
                direction: Direction::Out,
                node_id: *e,
                weight: w.as_ref().map(|x| *x),
            };

            graph.add_edge(*e, edge_info_ev);
            graph.add_edge(*v, edge_info_ve);
        }

        graph
    }

    fn directed(&self) -> bool {
        self.directed
    }

    fn num_nodes(&self) -> usize {
        self.num_nodes
    }

    fn num_edges(&self) -> usize {
        // self.n_edges.get()
        unimplemented!();
    }

    fn num_edges_directed(&self) -> usize {
        unimplemented!();
        // let mut sum = 0;
        // for (_, v) in self.vertices.borrow().iter() {
        //     sum += v.out_edges.len();
        // }
        // sum
    }

    fn out_degree(&self, v: NodeId) -> usize {
        let guard = &epoch::pin();
        if let Some(found) = self.find_vertex(v) {
            let mut count = 0;
            let edges = found.get().list.as_ref().unwrap().iter(guard);
            // while let Some(edge) = edges.next() {
            //     if let Some(present_edge) =  edge.value() {
            //         match present_edge.direction {
            //             Direction::Out => {
            //                 count += 1;
            //             },
            //             _ => {}
            //         }
            //     }
            // }
            count
        } else {
            panic!("Vertex not found");
        }
    }

    fn in_degree(&self, v: NodeId) -> usize {
        unimplemented!();
        // if let Some(found) = self.find_vertex(v) {
        //     let mut count = 0;
        //     let guard = &epoch::pin();
        //     let edges = found.get().list.as_ref().unwrap().iter(guard);
        //     while let Some(edge) = edges.next() {
        //         if let Some(present_edge) =  edge.value() {
        //             match present_edge.direction {
        //                 Direction::In => {
        //                     count += 1;
        //                 },
        //                 _ => {}
        //             }
        //         }
        //     }
        //     count
        // } else {
        //     panic!("Vertex not found");
        // }
    }

    fn out_neigh(&self, v: NodeId) -> Range<E> {
        unimplemented!();
        // if let Some(found) = self.find_vertex(v) {
        //     let mut picked_edges = Vec::new();
        //     let guard = &epoch::pin();
        //     let edges = found.get().list.as_ref().unwrap();
        //     // edges.iter(guard)
        //     //     .filter(|e| {
        //     //         if let Some(present_edge) = e.value() {
        //     //             match present_edge.direction {
        //     //                 Direction::Out => {
        //     //                     return true;
        //     //                 },
        //     //                 _ => {
        //     //                     return false;
        //     //                 }
        //     //             }
        //     //         }
        //     //         false
        //     //     })
        //     //     .for_each(|e| {
        //     //         picked_edges.push(*e.value().unwrap())
        //     //     });

        //     Box::new(picked_edges.into_iter())
        // } else {
        //     panic!("Vertex not found");
        // }
    }

    fn in_neigh(&self, v: NodeId) -> Range<E> {
        unimplemented!();

        // if let Some(found) = self.get_vertex(v) {
        //     let mut edges = Vec::new();
        //     for edge in &self.vertices.borrow().get(found.index).unwrap().in_edges {
        //         edges.push(edge.clone());
        //     }
        //     edges.sort_by(|a, b| a.as_node().cmp(&b.as_node()));
        //     Box::new(edges.into_iter())
        // } else {
        //     panic!("Vertex not found");
        // }
    }

    fn print_stats(&self) {
        println!("---------- GRAPH ----------");
        println!("  Num Nodes          - {:?}", self.num_nodes());
        println!("  Num Edges          - {:?}", self.num_edges_directed());
        println!("---------------------------");
    }

    fn vertices(&self) -> Range<V> {
        unimplemented!();

        // let mut edges = Vec::new();
        // for (idx, edge) in self.vertices.borrow().iter() {
        //     edges.push(CustomIndex{ index: idx, weight: None });
        // }
        // edges.sort_by(|a, b| a.as_node().cmp(&b.as_node()));
        // Box::new(edges.into_iter())
    }

    fn replace_out_edges(&self, v: NodeId, edges: Vec<E>) {
        unimplemented!();

        // if let Some(found) = self.get_vertex(v) {
        //     let mut new_edges = HashSet::new();
        //     for e in edges {
        //         new_edges.insert(e);
        //     }
        //     self.vertices.borrow_mut().get_mut(found.index).unwrap().out_edges = new_edges;
        // }
    }

    fn replace_in_edges(&self, v: NodeId, edges: Vec<E>) {
        unimplemented!();

        // if let Some(found) = self.get_vertex(v) {
        //     let mut new_edges = HashSet::new();
        //     for e in edges {
        //         new_edges.insert(e);
        //     }
        //     self.vertices.borrow_mut().get_mut(found.index).unwrap().in_edges = new_edges;
        // }
    }

    fn old_bfs(&self, v: NodeId) {
        unimplemented!();
        // self.bfs(v, None);
    }
}
