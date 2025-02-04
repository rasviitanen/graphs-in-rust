use crate::graph::{CSRGraph, Range};
use crate::types::*;
use generational_arena::{Arena, Index};
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashSet, VecDeque};

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
    cache: RefCell<BTreeMap<NodeId, Index>>,
    n_edges: Cell<usize>,
    num_edges: usize,
}

impl<'a, T: Clone> CSRGraph<CustomIndex, CustomIndex> for Graph<T> {
    fn build_directed(num_nodes: usize, edge_list: &EdgeList) -> Self {
        let mut graph = Graph::new(true);
        for v in 0..num_nodes {
            graph.add_vertex(v, None);
        }

        for (v, e, w) in edge_list {
            graph.add_edge(*v, *e, w, true);
            graph.num_edges += 1;
        }

        graph
    }

    fn build_undirected(num_nodes: usize, edge_list: &EdgeList) -> Self {
        let mut graph = Graph::new(false);
        // println!("Building undirected, with {} nodes", num_nodes);
        for v in 0..num_nodes {
            graph.add_vertex(v, None);
        }
        for (v, e, w) in edge_list {
            graph.add_edge(*v, *e, w, false);
            graph.num_edges += 1;
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
        if self.directed {
            self.num_edges
        } else {
            self.num_edges * 2
        }
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

    fn op_add_vertex(&self, v: NodeId) {
        self.add_vertex(v, None);
    }

    fn op_add_edge(&self, v: NodeId, e: NodeId) {
        self.add_edge(v, e, &None, false);
    }

    fn op_delete_edge(&self, v: NodeId, e: NodeId) {
        self.get_vertex(v).map(|idx| {
            self.vertices.borrow_mut().get_mut(idx.index).map(|vertex| {
                self.get_vertex(e)
                    .map(|edge| vertex.out_edges.remove(&edge))
            });
        });
    }

    fn op_delete_vertex(&self, v: NodeId) {
        self.get_vertex(v).map(|idx| {
            self.vertices.borrow_mut().remove(idx.index);
        });
    }

    fn op_find_vertex(&self, v: NodeId) {
        self.find_vertex(v);
    }
}

impl<T> Graph<T> {
    pub fn new(directed: bool) -> Self {
        Self {
            vertices: RefCell::new(Arena::new()),
            directed,
            cache: RefCell::new(BTreeMap::new()),
            n_edges: Cell::new(0),
            num_edges: 0,
        }
    }

    pub fn add_vertex(&self, key: usize, value: Option<T>) -> Index {
        let node = ArenaNode::new(key, value);
        let index = self.vertices.borrow_mut().insert(node);
        self.cache.borrow_mut().insert(key, index);
        index
    }

    pub fn find_vertex(&self, node_id: usize) -> Option<Index> {
        // for (idx, node) in self.vertices.borrow().iter() {
        //     if node.node_id == node_id {
        //         return Some(idx);
        //     }
        // }

        // None
        self.cache.borrow().get(&node_id).map(|n| *n)
    }

    pub fn get_vertex(&self, node_id: usize) -> Option<CustomIndex> {
        // for (idx, node) in self.vertices.borrow().iter() {
        //     if node.node_id == node_id {
        //         return Some(CustomIndex {
        //             index: idx,
        //             weight: None,
        //         });
        //     }
        // }

        // None
        self.cache.borrow().get(&node_id).map(|n| CustomIndex {
            index: *n,
            weight: None,
        })
    }

    pub fn add_edge(&self, node1: usize, node2: usize, weight: &Option<Weight>, directed: bool) {
        if let (Some(vertex), Some(edge)) = (self.get_vertex(node1), self.get_vertex(node2)) {
            if !directed {
                self.vertices
                    .borrow_mut()
                    .get_mut(edge.index)
                    .map(|vx| {
                        vx.out_edges
                        .insert(CustomIndex {
                            index: vertex.index,
                            weight: weight.as_ref().map(|x| *x),
                        });}
                    );
            } else {
                self.vertices
                    .borrow_mut()
                    .get_mut(edge.index)
                    .map(|vx|{
                        vx.in_edges
                        .insert(CustomIndex {
                            index: vertex.index,
                            weight: weight.as_ref().map(|x| *x),
                        });}
                    );
            }
            if self
                .vertices
                .borrow_mut()
                .get_mut(vertex.index)
                .map(|vx|{
                    vx.out_edges
                    .insert(CustomIndex {
                        index: edge.index,
                        weight: weight.as_ref().map(|x| *x),
                    })}
                ).unwrap_or(false)
            {
                self.n_edges.update(|x| x + 1);
            }
        } else {
            // dbg!(node1, node2);
            // panic!("Could not add edge, one or both of the nodes you are trying to connect does not exist");
        }
    }

    pub fn bfs(&self, start: usize, goal: Option<usize>) -> usize {
        unimplemented!("NO OLD BFS");
    }
}
