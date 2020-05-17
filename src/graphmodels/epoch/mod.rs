mod adjlist;
mod lftt;
mod mdlist;

use crate::graphmodels::epoch::adjlist::AdjacencyList;
pub use crate::graphmodels::epoch::adjlist::{IterRefEntry, Node, RefEntry};

pub use crate::graphmodels::epoch::lftt::{OpType, ReturnCode};
use std::sync::atomic::Ordering::{Relaxed, SeqCst};

use epoch::{Atomic, Guard, Shared};
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::{BTreeMap, HashSet, VecDeque};
use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, RwLock};

use crate::graph::{CSRGraph, Range};
use crate::types::*;

#[derive(Clone, Copy)]
pub struct CustomNode(usize);

impl AsNode for CustomNode {
    #[inline]
    fn as_node(&self) -> NodeId {
        self.0
    }
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

#[derive(Copy, Clone, Debug)]
pub struct EdgeInfo {
    // pub vertex_ref: RefEntry<'a, 'a, T, Self>,
    pub node_id: NodeId,
    pub weight: Option<Weight>,
}

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

pub struct Graph<'a, T: Copy + Clone + Into<usize>> {
    inner: AdjacencyList<'a, T, EdgeInfo>,
    cache: Arc<RwLock<BTreeMap<NodeId, &'a Node<'a, T, E>>>>,
    directed: bool,
    num_nodes: usize,
    num_edges: usize,
}

type E = EdgeInfo;

impl<'a, T: 'a + Copy + Clone + Into<usize>> Graph<'a, T> {
    pub fn new(size_hint: i64, directed: bool) -> Self {
        Self {
            inner: AdjacencyList::new(size_hint),
            cache: Arc::new(RwLock::new(BTreeMap::new())),
            directed,
            num_nodes: 0,
            num_edges: 0,
        }
    }

    pub fn execute_ops<'t>(
        &'t self,
        ops: Vec<OpType<'a, T, E>>,
    ) -> std::sync::mpsc::Receiver<ReturnCode<Atomic<Node<'a, T, E>>>> {
        self.inner.txn(ops).execute()
    }

    pub fn add_vertex<'t>(
        &'t self,
        key: usize,
        value: Option<T>,
    ) -> Option<(usize, Atomic<Node<'a, T, E>>)> {
        let op = OpType::Insert(key, value);
        let insertion_txn = self.inner.txn(vec![op]).execute();

        if let Ok(ReturnCode::Inserted(entry)) = insertion_txn.recv() {
            Some((key, entry))
        } else {
            None
        }
    }

    pub fn add_edge(&self, parent_id: usize, edge_info: E, direction_in: bool) {
        let op = OpType::InsertEdge(parent_id, edge_info.node_id, Some(edge_info), direction_in);
        let insert_edge_txn = self.inner.txn(vec![op]);
        insert_edge_txn.execute().recv().expect("Txn failed");
    }

    pub fn add_empty_edge(&self, parent: usize, child: usize, direction_in: bool) {
        let op = OpType::InsertEdge(parent, child, None, direction_in);
        let insert_edge_txn = self.inner.txn(vec![op]);
        insert_edge_txn.execute().recv().expect("Txn failed");
    }

    pub fn connect<'t>(parent: &Node<T, E>, child: E, direction_in: bool) {
        unsafe {
            AdjacencyList::connect(parent, child.node_id, child, direction_in);
        }
    }

    // pub fn connect_with_id<'t>(&self, v: usize, e: usize) {
    //     let edge_info_ev = EdgeInfo {
    //         direction: Direction::In,
    //         node_id: v,
    //         weight: None,
    //     };

    //     let edge_info_ve = EdgeInfo {
    //         direction: Direction::Out,
    //         node_id: e,
    //         weight: None,
    //     };

    //     if let (Some(en), Some(vn)) = (self.cache.read().unwrap().get(&e), self.cache.read().unwrap().get(&v)) {
    //         Self::connect(en, edge_info_ev);
    //         Self::connect(vn, edge_info_ve);
    //     }
    // }

    pub fn iter_vertices<'t, 'g>(&'a self, guard: &'g Guard) -> IterRefEntry<'a, 't, 'g, T, E> {
        self.inner.iter(guard)
    }

    pub fn find_vertex<'t>(&'t self, key: usize) -> Option<Atomic<Node<'a, T, E>>> {
        let op = OpType::Find(key);
        let find_txn = self.inner.txn(vec![op]);
        let res = find_txn.execute();

        if let ReturnCode::Found(entry) = res.recv().unwrap() {
            Some(entry)
        } else {
            None
        }
    }

    pub fn delete_vertex<'t>(&'t self, key: usize) -> Option<Atomic<Node<'a, T, E>>> {
        let op = OpType::Delete(key);
        let insertion_txn = self.inner.txn(vec![op]).execute();

        if let Ok(ReturnCode::Deleted(entry)) = insertion_txn.recv() {
            Some(entry)
        } else {
            None
        }
    }

    pub fn delete_edge<'t>(
        &'t self,
        parent: usize,
        edge: usize,
        direction_in: bool,
    ) -> Result<(), ()> {
        let op = OpType::DeleteEdge(parent, edge, direction_in);
        let insertion_txn = self.inner.txn(vec![op]).execute();

        if let Ok(ReturnCode::Success) = insertion_txn.recv() {
            Ok(())
        } else {
            Err(())
        }
    }
}

impl<'a> CSRGraph<CustomNode, EdgeInfo> for Graph<'_, usize> {
    fn build_directed(num_nodes: usize, edge_list: &EdgeList) -> Self {
        let mut graph = Graph::new(num_nodes as i64, true);
        let guard = unsafe { &*(&epoch::pin() as *const _) };
        for v in 1..num_nodes {
            let inserted = graph.add_vertex(v, None);
            graph
                .cache
                .write()
                .expect("Could not write")
                .insert(v, unsafe {
                    inserted.unwrap().1.load(SeqCst, guard).deref()
                });
        }

        graph.num_nodes = num_nodes;

        for (v, e, w) in edge_list {
            if *v == 0 || *e == 0 {
                // Our datastructure cannot handle id 0
                continue;
            }

            let edge_info_ev = EdgeInfo {
                node_id: *v,
                weight: w.as_ref().map(|x| *x),
            };

            let edge_info_ve = EdgeInfo {
                node_id: *e,
                weight: w.as_ref().map(|x| *x),
            };

            graph.num_edges += 1;

            if let (Some(en), Some(vn)) = (
                graph.cache.read().unwrap().get(e),
                graph.cache.read().unwrap().get(v),
            ) {
                Self::connect(en, edge_info_ev, true);
                Self::connect(vn, edge_info_ve, false);
            }
        }

        graph
    }

    fn build_undirected(num_nodes: usize, edge_list: &EdgeList) -> Self {
        let mut graph = Graph::new(num_nodes as i64, false);
        let guard = unsafe { &*(&epoch::pin() as *const _) };

        for v in 1..num_nodes {
            let inserted = graph.add_vertex(v, None);
            graph
                .cache
                .write()
                .expect("Could not write")
                .insert(v, unsafe {
                    inserted.unwrap().1.load(SeqCst, guard).deref()
                });
        }

        graph.num_nodes = num_nodes;

        for (v, e, w) in edge_list {
            if *v == 0 || *e == 0 {
                // Our datastructure cannot handle id 0
                continue;
            }

            let edge_info_ev = EdgeInfo {
                node_id: *v,
                weight: w.as_ref().map(|x| *x),
            };

            let edge_info_ve = EdgeInfo {
                node_id: *e,
                weight: w.as_ref().map(|x| *x),
            };

            graph.num_edges += 1;

            if let (Some(en), Some(vn)) = (
                graph.cache.read().unwrap().get(e),
                graph.cache.read().unwrap().get(v),
            ) {
                Self::connect(en, edge_info_ev, false);
                Self::connect(vn, edge_info_ve, false);
            }
        }

        graph
    }

    #[inline]
    fn directed(&self) -> bool {
        self.directed
    }

    #[inline]
    fn num_nodes(&self) -> usize {
        self.num_nodes
    }

    fn num_edges(&self) -> usize {
        // self.n_edges.get()
        unimplemented!();
    }

    #[inline]
    fn num_edges_directed(&self) -> usize {
        if self.directed {
            self.num_edges
        } else {
            self.num_edges * 2
        }
    }

    fn out_degree(&self, v: NodeId) -> usize {
        if v == 0 {
            // Our datastructure cannot handle id 0
            return 0;
        }

        let guard = &epoch::pin();
        if let Some(found) = self.cache.read().unwrap().get(&v) {
            // self.find_vertex(v) {
            found.out_edges.as_ref().unwrap().len()
        // let mut count = 0;
        // let mut edges = found.list.as_ref().unwrap().iter(guard);
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
        // count
        } else {
            panic!("Vertex not found");
        }
    }

    fn in_degree(&self, v: NodeId) -> usize {
        if v == 0 {
            // Our datastructure cannot handle id 0
            return 0;
        }
        let guard = &epoch::pin();
        if let Some(found) = self.cache.read().unwrap().get(&v) {
            // self.find_vertex(v) {
            // let mut count = 0;
            // let guard = &epoch::pin();
            found.in_edges.as_ref().unwrap().len()
        // let mut edges = found.list.as_ref().unwrap().iter(guard);
        // while let Some(edge) = edges.next() {
        //     if let Some(present_edge) =  edge.value() {
        //         match present_edge.direction {
        //             Direction::In => {
        //                 count += 1;
        //             },
        //             _ => {}
        //         }
        //     }
        // }
        // count
        } else {
            panic!("Vertex not found");
        }
    }

    fn out_neigh(&self, v: NodeId) -> Range<E> {
        if v == 0 || v == 18446744073709551615 {
            // Our datastructure cannot handle id 0
            return Box::new(Vec::new().into_iter());
        }

        // println!("GETTING OUT NEIGH OF {}", v);

        let guard = unsafe { &*(&epoch::pin() as *const _) };
        if let Some(found) = self.cache.read().unwrap().get(&v) {
            // self.find_vertex(v) {
            let edges = found.out_edges.as_ref().unwrap();
            let picked_edges = edges.iter(guard).map(|e| *e.value().unwrap());
            // picked_edges
            Box::new(picked_edges)
        } else {
            panic!("Vertex not found");
        }
    }

    fn in_neigh(&self, v: NodeId) -> Range<E> {
        if v == 0 {
            // Our datastructure cannot handle id 0
            return Box::new(Vec::new().into_iter());
        }

        let guard = unsafe { &*(&epoch::pin() as *const _) };
        if let Some(found) = self.cache.read().unwrap().get(&v) {
            // self.find_vertex(v) {
            let edges = found.in_edges.as_ref().unwrap();
            let picked_edges = edges.iter(guard).map(|e| *e.value().unwrap());
            // picked_edges
            Box::new(picked_edges)
        } else {
            panic!("Vertex not found");
        }
    }

    fn print_stats(&self) {
        println!("---------- GRAPH ----------");
        println!("  Num Nodes          - {:?}", self.num_nodes());
        println!("  Num Edges          - {:?}", self.num_edges_directed());
        println!("---------------------------");
    }

    fn vertices(&self) -> Range<CustomNode> {
        let guard = unsafe { &*(&epoch::pin() as *const _) };
        let iter = self.inner.iter(guard).map(|v| CustomNode(v.get().key));
        Box::new(iter)
    }

    fn replace_out_edges(&self, v: NodeId, edges: Vec<E>) {
        // unimplemented!();

        // if let Some(found) = self.get_vertex(v) {
        //     let mut new_edges = HashSet::new();
        //     for e in edges {
        //         new_edges.insert(e);
        //     }
        //     self.vertices.borrow_mut().get_mut(found.index).unwrap().out_edges = new_edges;
        // }
    }

    fn replace_in_edges(&self, v: NodeId, edges: Vec<E>) {
        // unimplemented!();

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

    fn op_add_vertex(&self, v: NodeId) {
        self.add_vertex(v, None);
    }

    fn op_add_edge(&self, v: NodeId, e: NodeId) {
        let edge_info = EdgeInfo {
            node_id: e,
            weight: None,
        };

        let op = OpType::InsertEdge(v, e, Some(edge_info), false);
        let insert_edge_txn = self.inner.txn(vec![op]);
        insert_edge_txn.execute();
    }

    fn op_delete_edge(&self, v: NodeId, e: NodeId) {
        self.delete_edge(v, e, false);
    }

    fn op_delete_vertex(&self, v: NodeId) {
        let op = OpType::Delete(v);
        let find_txn = self.inner.txn(vec![op]);
        let res = find_txn.execute();
    }

    fn op_find_vertex(&self, v: NodeId) {
        let op = OpType::Find(v);
        let find_txn = self.inner.txn(vec![op]);
        let res = find_txn.execute();
    }
}
