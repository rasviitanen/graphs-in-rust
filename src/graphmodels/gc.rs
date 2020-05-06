use crate::graph::{CSRGraph, Range};
use crate::types::*;
use gc::{Finalize, Gc, GcCell, Trace};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Trace, Finalize)]
pub struct Node<T: 'static + Trace> {
    node_id: NodeId,
    value: Option<T>,
    weight: Option<usize>,
    in_edges: HashMap<usize, Gc<GcCell<Self>>>,
    out_edges: HashMap<usize, Gc<GcCell<Self>>>,
}

impl<T: Trace> AsNode for Gc<GcCell<Node<T>>> {
    fn as_node(&self) -> NodeId {
        self.borrow().node_id
    }
}

impl<T: Trace> WeightedEdge for Gc<GcCell<Node<T>>> {
    fn get_weight(&self) -> usize {
        self.borrow()
            .weight
            .expect("Weights must be assigned before used")
    }

    fn set_weight(&mut self, weight: usize) {
        self.borrow_mut().weight.replace(weight);
    }
}

impl<T: Trace> Node<T> {
    pub fn new(node_id: NodeId, value: Option<T>) -> Gc<GcCell<Node<T>>> {
        let node = Node {
            node_id,
            value,
            weight: None,
            in_edges: HashMap::new(),
            out_edges: HashMap::new(),
        };

        Gc::new(GcCell::new(node))
    }

    fn add_in_edge(
        this: &Gc<GcCell<Node<T>>>,
        edge: &Gc<GcCell<Node<T>>>,
        weight: &Option<usize>,
    ) -> bool {
        let node_id = edge.borrow().node_id;

        // Disable self-edges
        if this.borrow().node_id == node_id {
            return false;
        }

        this.borrow_mut()
            .in_edges
            .insert(node_id, {
                let mut edge = Gc::clone(edge);
                if let Some(w) = weight {
                    edge.set_weight(*w);
                }
                edge
            })
            .is_none()
    }

    fn add_out_edge(
        this: &Gc<GcCell<Node<T>>>,
        edge: &Gc<GcCell<Node<T>>>,
        weight: &Option<usize>,
    ) -> bool {
        let node_id = edge.borrow().node_id;

        // Disable self-edges
        if this.borrow().node_id == node_id {
            return false;
        }

        this.borrow_mut()
            .out_edges
            .insert(node_id, {
                let mut edge = Gc::clone(edge);
                if let Some(w) = weight {
                    edge.set_weight(*w);
                }
                edge
            })
            .is_none()
    }
}

pub struct Graph<T: Trace + 'static> {
    vertices: RefCell<HashMap<usize, Gc<GcCell<Node<T>>>>>,
    n_edges: Cell<usize>,
    directed: bool,
}

impl<'a, T: Clone + Trace> CSRGraph<'a, Gc<GcCell<Node<T>>>, Gc<GcCell<Node<T>>>> for Graph<T> {
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
        println!("Building undirected, with {} nodes", num_nodes);
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
        for v in self.vertices() {
            sum += v.borrow().out_edges.len();
        }
        sum
    }

    fn out_degree(&self, v: NodeId) -> usize {
        if let Some(found) = self.vertices.borrow().get(&v) {
            found.borrow().out_edges.len()
        } else {
            0
        }
    }

    fn in_degree(&self, v: NodeId) -> usize {
        println!("Graph inversion is probably disabled... in in_degree()");
        if let Some(found) = self.vertices.borrow().get(&v) {
            found.borrow().in_edges.len()
        } else {
            panic!("Vertex not found");
        }
    }

    fn out_neigh(&self, v: NodeId) -> Range<Gc<GcCell<Node<T>>>> {
        if let Some(vertex) = self.vertices.borrow().get(&v) {
            let mut edges = Vec::new();
            for edge in vertex.borrow().out_edges.values() {
                edges.push(Gc::clone(&edge));
            }
            edges.sort_by(|a, b| a.as_node().cmp(&b.as_node()));
            Box::new(edges.into_iter())
        } else {
            panic!("Vertex not found");
        }
    }

    fn in_neigh(&self, v: NodeId) -> Range<Gc<GcCell<Node<T>>>> {
        if let Some(vertex) = self.vertices.borrow().get(&v) {
            let mut edges = Vec::new();
            for edge in vertex.borrow().in_edges.values() {
                edges.push(Gc::clone(&edge));
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

    fn vertices(&self) -> Range<Gc<GcCell<Node<T>>>> {
        let mut edges = Vec::new();
        for edge in self.vertices.borrow().values() {
            edges.push(Gc::clone(&edge));
        }
        edges.sort_by(|a, b| a.as_node().cmp(&b.as_node()));
        Box::new(edges.into_iter())
    }

    fn replace_out_edges(&self, v: NodeId, edges: Vec<Gc<GcCell<Node<T>>>>) {
        if let Some(vertex) = self.vertices.borrow().get(&v) {
            let mut new_edges = HashMap::new();
            for e in edges {
                new_edges.insert(e.as_node(), e);
            }
            vertex.borrow_mut().out_edges = new_edges;
        }
    }

    fn replace_in_edges(&self, v: NodeId, edges: Vec<Gc<GcCell<Node<T>>>>) {
        if let Some(vertex) = self.vertices.borrow().get(&v) {
            let mut new_edges = HashMap::new();
            for e in edges {
                new_edges.insert(e.as_node(), e);
            }
            vertex.borrow_mut().in_edges = new_edges;
        }
    }

    fn old_bfs(&self, v: NodeId) {
        self.bfs(v, None);
    }
}

impl<T: Trace> Graph<T> {
    pub fn new(directed: bool) -> Self {
        Graph {
            vertices: RefCell::new(HashMap::new()),
            n_edges: Cell::new(0),
            directed,
        }
    }

    pub fn find_vertex(&self, vertex: usize) -> Option<Gc<GcCell<Node<T>>>> {
        self.vertices.borrow().get(&vertex).map(|v| Gc::clone(v))
    }

    pub fn add_vertex(&self, node_id: usize, value: Option<T>) -> Gc<GcCell<Node<T>>> {
        let new_node = Node::new(node_id, value);
        self.vertices
            .borrow_mut()
            .entry(node_id)
            .or_insert(Gc::clone(&new_node));
        new_node
    }

    pub fn add_edge(&self, vertex: usize, edge: usize, weight: &Option<usize>, directed: bool) {
        if let (Some(vertex_node), Some(edge_node)) = (
            self.vertices.borrow().get(&vertex),
            self.vertices.borrow().get(&edge),
        ) {
            if !directed {
                Node::add_out_edge(&edge_node, &vertex_node, weight);
            } else {
                Node::add_in_edge(&edge_node, &vertex_node, weight);
            }

            if Node::add_out_edge(&vertex_node, &edge_node, weight) {
                self.n_edges.update(|x| x + 1);
            }
        } else {
            panic!("Could not add edge, one or both of the nodes you are trying to connect does not exist");
        }
    }

    pub fn connect(
        &self,
        vertex_node: &Gc<GcCell<Node<T>>>,
        edge_node: &Gc<GcCell<Node<T>>>,
        weight: &Option<usize>,
        directed: bool,
    ) {
        if !directed {
            Node::add_out_edge(&edge_node, &vertex_node, weight);
        } else {
            Node::add_in_edge(&edge_node, &vertex_node, weight);
        }

        if Node::add_out_edge(&vertex_node, &edge_node, weight) {
            self.n_edges.update(|x| x + 1);
        }
    }

    pub fn bfs(&self, start: usize, goal: Option<usize>) -> usize {
        let mut queue = VecDeque::new();
        let mut discovered = HashSet::new();

        let start = self.find_vertex(start).unwrap();
        discovered.insert(start.borrow().node_id);
        queue.push_back(Gc::clone(&start));

        while let Some(node) = queue.pop_front() {
            let locked_node = node.borrow();
            for edge in locked_node.out_edges.values() {
                let edge_node_id = edge.borrow().node_id;

                if goal == Some(edge_node_id) {
                    return discovered.len();
                }

                if !discovered.contains(&edge_node_id) {
                    discovered.insert(edge_node_id);
                    queue.push_back(Gc::clone(&edge));
                }
            }
        }

        discovered.len()
    }
}
