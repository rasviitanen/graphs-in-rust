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

pub trait Edge<'a, T> {
    fn key(&self) -> usize;
    fn as_node(&self) -> Shared<'a, Node<'a, T, Self>>
    where
        Self: std::marker::Sized;
}

pub trait Vertex {
    fn key(&self) -> usize;
}

unsafe impl<'a, T: Send + Sync> Send for EdgeInfo<'a, T> {}
unsafe impl<'a, T: Send + Sync> Sync for EdgeInfo<'a, T> {}

#[derive(Clone)]
pub struct EdgeInfo<'a, T> {
    pub vertex_ref: RefEntry<'a, 'a, T, Self>,
    weight: Option<Weight>,
}

impl<'a, T> EdgeInfo<'a, T> {
    pub fn new(vertex_ref: RefEntry<'a, 'a, T, Self>) -> Self {
        EdgeInfo {
            vertex_ref,
            weight: None,
        }
    }
}

impl<T> WeightedEdge for EdgeInfo<'_, T> {
    fn get_weight(&self) -> usize {
        self.weight.expect("Weights must be assigned before used")
    }

    fn set_weight(&mut self, weight: usize) {
        self.weight.replace(weight);
    }
}

impl<T> AsNode for EdgeInfo<'_, T> {
    fn as_node(&self) -> NodeId {
        self.vertex_ref.get().key
    }
}

impl<'a, T> Edge<'a, T> for EdgeInfo<'a, T> {
    #[inline]
    fn key(&self) -> usize {
        unsafe { self.vertex_ref.node.as_ref().unwrap().key }
    }

    #[inline]
    fn as_node(&self) -> Shared<'a, Node<'a, T, Self>> {
        self.vertex_ref.node
    }
}

pub struct Graph<'a, T: Clone, E: Clone + Edge<'a, T>> {
    inner: AdjacencyList<'a, T, E>,
    directed: bool,
}

impl<'a, T: 'a + Clone, E: 'a + Clone + Edge<'a, T>> Graph<'a, T, E> {
    pub fn new(size_hint: i64, directed: bool) -> Self {
        Self {
            inner: AdjacencyList::new(size_hint),
            directed,
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

    pub fn add_edge<'t>(&'t self, parent_id: usize, edge_info: E) {
        let op = OpType::InsertEdge(parent_id, edge_info.key(), Some(edge_info));
        let insert_edge_txn = self.inner.txn(vec![op]);
        insert_edge_txn.execute().recv().expect("Txn failed");
    }

    pub fn add_empty_edge<'t>(&'t self, parent: usize, child: usize) {
        let op = OpType::InsertEdge(parent, child, None);
        let insert_edge_txn = self.inner.txn(vec![op]);
        insert_edge_txn.execute().recv().expect("Txn failed");
    }

    pub fn connec<'t>(parent: &RefEntry<'a, 't, T, E>, child: E) {
        unsafe {
            AdjacencyList::connect(parent, child.key(), child);
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

    pub fn delete_edge(&'a self, parent: usize, edge: usize) -> Result<(), ()> {
        let op = OpType::DeleteEdge(parent, edge);
        let insertion_txn = self.inner.txn(vec![op]).execute();

        if let Ok(ReturnCode::Success) = insertion_txn.recv() {
            Ok(())
        } else {
            Err(())
        }
    }

    pub fn bfs(&'a self, start: usize, goal: Option<usize>) -> usize {
        let guard = &epoch::pin();
        let mut queue = VecDeque::new();
        let mut discovered = HashSet::new();
        discovered.insert(start);

        let start = self.find_vertex(start).expect("No start node found");
        queue.push_back(start.node);
        unsafe {
            while let Some(node) = queue.pop_front() {
                let node_ref = node.as_ref().expect("Child is NULL");
                let mut child_entries = node_ref.list.as_ref().unwrap().iter(guard);
                while let Some(child) = child_entries.next() {
                    if let Some(child_ref) = child.value().as_ref() {
                        if goal == Some(child_ref.key()) {
                            return discovered.len();
                        }

                        if !discovered.contains(&child_ref.key()) {
                            discovered.insert(child_ref.key());
                            let value = child.value();
                            queue.push_back(value.unwrap().as_node());
                        }
                    }
                }
            }
        }

        discovered.len()
    }
}

impl<'a, V: Clone, E: Clone + Edge<'a, V>> CSRGraph<'a, V, E> for Graph<'a, V, E> {
    fn build_directed(num_nodes: usize, edge_list: &EdgeList) -> Self {
        let graph = Graph::new(num_nodes as i64, true);

        for v in 0..num_nodes {
            graph.add_vertex(v, None);
        }

        for (v, e, w) in edge_list {
            // graph.add_edge(*v, *e, w, true)
            unimplemented!();
        }

        graph
    }

    fn build_undirected(num_nodes: usize, edge_list: &EdgeList) -> Self {
        let graph = Graph::new(num_nodes as i64, false);

        for v in 0..num_nodes {
            graph.add_vertex(v, None);
        }
        for (v, e, w) in edge_list {
            // graph.add_edge(*v, *e, w, false);
            unimplemented!();
        }

        graph
    }

    fn directed(&self) -> bool {
        self.directed
    }

    fn num_nodes(&self) -> usize {
        // self.vertices.borrow().len()
        unimplemented!();
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
        unimplemented!();

        // if let Some(found) = self.get_vertex(v) {
        //     self.vertices.borrow().get(found.index).unwrap().out_edges.len()
        // } else {
        //     panic!("Vertex not found");
        // }
    }

    fn in_degree(&self, v: NodeId) -> usize {
        unimplemented!();

        // println!("Graph inversion is probably disabled... in in_degree()");
        // if let Some(found) = self.get_vertex(v) {
        //     self.vertices.borrow().get(found.index).unwrap().in_edges.len()
        // } else {
        //     panic!("Vertex not found");
        // }
    }

    fn out_neigh(&self, v: NodeId) -> Range<E> {
        unimplemented!();

        // if let Some(found) = self.get_vertex(v) {
        //     let mut edges = Vec::new();
        //     for edge in &self.vertices.borrow().get(found.index).unwrap().out_edges {
        //         edges.push(edge.clone());
        //     }
        //     edges.sort_by(|a, b| a.as_node().cmp(&b.as_node()));
        //     Box::new(edges.into_iter())
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
