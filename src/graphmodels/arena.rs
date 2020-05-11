use crate::graph::{CSRGraph, Range};
use crate::types::*;
use generational_arena::{Arena, Index};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet, VecDeque};

type Weight = usize;

#[derive(Clone, Hash, Eq, PartialEq)]
pub struct CustomIndex {
    index: Index,
    weight: Option<Weight>,
}

impl AsNode for CustomIndex {
    fn as_node(&self) -> NodeId {
        self.index.into_raw_parts().0
    }
}

impl WeightedEdge for CustomIndex {
    fn get_weight(&self) -> Weight {
        self.weight.expect("Weights must be assigned before used")
    }

    fn set_weight(&mut self, weight: Weight) {
        self.weight.replace(weight);
    }
}

pub struct ArenaNode<T> {
    node_id: usize,
    value: Option<T>,
    in_edges: HashSet<CustomIndex>,
    out_edges: HashSet<CustomIndex>,
}

impl<T> ArenaNode<T> {
    fn new(node_id: usize, value: Option<T>) -> Self {
        Self {
            node_id,
            value,
            in_edges: HashSet::new(),
            out_edges: HashSet::new(),
        }
    }
}

pub struct Graph<T> {
    vertices: RefCell<Arena<ArenaNode<T>>>,
    directed: bool,
    n_edges: Cell<usize>,
}

impl<'a, T: Clone> CSRGraph<CustomIndex, CustomIndex> for Graph<T> {
    fn build_directed(num_nodes: usize, edge_list: &EdgeList) -> Self {
        let graph = Graph::new(true);
        for v in 0..num_nodes {
            graph.add_vertex(v, None);
        }

        for (v, e, w) in edge_list {
            graph.add_edge(*v, *e, w, true)
        }
        graph
    }

    fn build_undirected(num_nodes: usize, edge_list: &EdgeList) -> Self {
        let graph = Graph::new(false);
        // println!("Building undirected, with {} nodes", num_nodes);
        for v in 0..num_nodes {
            graph.add_vertex(v, None);
        }
        for (v, e, w) in edge_list {
            graph.add_edge(*v, *e, w, false);
        }

        graph
    }

    fn directed(&self) -> bool {
        self.directed
    }

    fn num_nodes(&self) -> usize {
        self.vertices.borrow().len()
    }

    fn num_edges(&self) -> usize {
        self.n_edges.get()
    }

    fn num_edges_directed(&self) -> usize {
        let mut sum = 0;
        for (_, v) in self.vertices.borrow().iter() {
            sum += v.out_edges.len();
        }
        sum
    }

    fn out_degree(&self, v: NodeId) -> usize {
        if let Some(found) = self.get_vertex(v) {
            self.vertices
                .borrow()
                .get(found.index)
                .unwrap()
                .out_edges
                .len()
        } else {
            panic!("Vertex not found");
        }
    }

    fn in_degree(&self, v: NodeId) -> usize {
        println!("Graph inversion is probably disabled... in in_degree()");
        if let Some(found) = self.get_vertex(v) {
            self.vertices
                .borrow()
                .get(found.index)
                .unwrap()
                .in_edges
                .len()
        } else {
            panic!("Vertex not found");
        }
    }

    fn out_neigh(&self, v: NodeId) -> Range<CustomIndex> {
        if let Some(found) = self.get_vertex(v) {
            let mut edges = Vec::new();
            for edge in &self.vertices.borrow().get(found.index).unwrap().out_edges {
                edges.push(edge.clone());
            }
            edges.sort_by(|a, b| a.as_node().cmp(&b.as_node()));
            Box::new(edges.into_iter())
        } else {
            panic!("Vertex not found");
        }
    }

    fn in_neigh(&self, v: NodeId) -> Range<CustomIndex> {
        if let Some(found) = self.get_vertex(v) {
            let mut edges = Vec::new();
            for edge in &self.vertices.borrow().get(found.index).unwrap().in_edges {
                edges.push(edge.clone());
            }
            edges.sort_by(|a, b| a.as_node().cmp(&b.as_node()));
            Box::new(edges.into_iter())
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

    fn vertices(&self) -> Range<CustomIndex> {
        let mut edges = Vec::new();
        for (idx, edge) in self.vertices.borrow().iter() {
            edges.push(CustomIndex {
                index: idx,
                weight: None,
            });
        }
        edges.sort_by(|a, b| a.as_node().cmp(&b.as_node()));
        Box::new(edges.into_iter())
    }

    fn replace_out_edges(&self, v: NodeId, edges: Vec<CustomIndex>) {
        if let Some(found) = self.get_vertex(v) {
            let mut new_edges = HashSet::new();
            for e in edges {
                new_edges.insert(e);
            }
            self.vertices
                .borrow_mut()
                .get_mut(found.index)
                .unwrap()
                .out_edges = new_edges;
        }
    }

    fn replace_in_edges(&self, v: NodeId, edges: Vec<CustomIndex>) {
        if let Some(found) = self.get_vertex(v) {
            let mut new_edges = HashSet::new();
            for e in edges {
                new_edges.insert(e);
            }
            self.vertices
                .borrow_mut()
                .get_mut(found.index)
                .unwrap()
                .in_edges = new_edges;
        }
    }

    fn old_bfs(&self, v: NodeId) {
        self.bfs(v, None);
    }
}

impl<T> Graph<T> {
    pub fn new(directed: bool) -> Self {
        Self {
            vertices: RefCell::new(Arena::new()),
            directed,
            n_edges: Cell::new(0),
        }
    }

    pub fn add_vertex(&self, key: usize, value: Option<T>) -> Index {
        let node = ArenaNode::new(key, value);
        self.vertices.borrow_mut().insert(node)
    }

    pub fn find_vertex(&self, node_id: usize) -> Option<Index> {
        for (idx, node) in self.vertices.borrow().iter() {
            if node.node_id == node_id {
                return Some(idx);
            }
        }

        None
    }

    pub fn get_vertex(&self, node_id: usize) -> Option<CustomIndex> {
        for (idx, node) in self.vertices.borrow().iter() {
            if node.node_id == node_id {
                return Some(CustomIndex {
                    index: idx,
                    weight: None,
                });
            }
        }

        None
    }

    pub fn add_edge(&self, node1: usize, node2: usize, weight: &Option<Weight>, directed: bool) {
        if let (Some(vertex), Some(edge)) = (self.get_vertex(node1), self.get_vertex(node2)) {
            if !directed {
                self.vertices
                    .borrow_mut()
                    .get_mut(edge.index)
                    .unwrap()
                    .out_edges
                    .insert(CustomIndex {
                        index: vertex.index,
                        weight: weight.as_ref().map(|x| *x),
                    });
            } else {
                self.vertices
                    .borrow_mut()
                    .get_mut(edge.index)
                    .unwrap()
                    .in_edges
                    .insert(CustomIndex {
                        index: vertex.index,
                        weight: weight.as_ref().map(|x| *x),
                    });
            }
            if self
                .vertices
                .borrow_mut()
                .get_mut(vertex.index)
                .unwrap()
                .out_edges
                .insert(CustomIndex {
                    index: edge.index,
                    weight: weight.as_ref().map(|x| *x),
                })
            {
                self.n_edges.update(|x| x + 1);
            }
        } else {
            panic!("Could not add edge, one or both of the nodes you are trying to connect does not exist");
        }
    }

    pub fn bfs(&self, start: usize, goal: Option<usize>) -> usize {
        unimplemented!("NO OLD BFS");
    }
}
