use crate::graph::{CSRGraph, Range};
use crate::types::*;
use std::cell::Cell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct WrappedNode<T> {
    inner: Arc<RwLock<Node<T>>>,
    weight: Option<usize>,
}

impl<T> WeightedEdge for WrappedNode<T> {
    fn get_weight(&self) -> usize {
        self.weight.expect("Weights must be assigned before used")
    }

    fn set_weight(&mut self, weight: usize) {
        self.weight.replace(weight);
    }
}

impl<T> AsNode for WrappedNode<T> {
    fn as_node(&self) -> NodeId {
        self.inner.read().expect("Could not read").node_id
    }
}

impl<T> std::ops::Deref for WrappedNode<T> {
    type Target = Arc<RwLock<Node<T>>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> WrappedNode<T> {
    pub fn from_node(node: Arc<RwLock<Node<T>>>, weight: &Option<usize>) -> Self {
        Self {
            inner: node,
            weight: weight.as_ref().map(|x| *x),
        }
    }
}

pub struct Node<T> {
    node_id: NodeId,
    value: Option<T>,
    in_edges: HashMap<usize, WrappedNode<T>>,
    out_edges: HashMap<usize, WrappedNode<T>>,
}

impl<T> Node<T> {
    pub fn new(node_id: NodeId, value: Option<T>) -> Arc<RwLock<Node<T>>> {
        let node = Node {
            node_id,
            value,
            in_edges: HashMap::new(),
            out_edges: HashMap::new(),
        };

        Arc::new(RwLock::new(node))
    }

    fn add_in_edge(
        this: &Arc<RwLock<Node<T>>>,
        edge: &Arc<RwLock<Node<T>>>,
        weight: &Option<usize>,
    ) -> bool {
        let node_id = edge.read().expect("Could not read").node_id;

        // Disable self-edges
        if this.read().expect("Could not read").node_id == node_id {
            return false;
        }

        this.write()
            .expect("Could not write")
            .in_edges
            .insert(node_id, WrappedNode::from_node(Arc::clone(edge), weight))
            .is_none()
    }

    fn add_out_edge(
        this: &Arc<RwLock<Node<T>>>,
        edge: &Arc<RwLock<Node<T>>>,
        weight: &Option<usize>,
    ) -> bool {
        let node_id = edge.read().expect("Could not read").node_id;

        // Disable self-edges
        if this.read().expect("Could not read").node_id == node_id {
            return false;
        }

        this.write()
            .expect("Could not write")
            .out_edges
            .insert(node_id, WrappedNode::from_node(Arc::clone(edge), weight))
            .is_none()
    }
}

pub struct Graph<T> {
    vertices: RwLock<HashMap<usize, WrappedNode<T>>>,
    num_nodes: usize,
    num_edges_directed: usize,
    num_edges_undirected: usize,
    directed: bool,
}

impl<'a, T: 'a + Clone> CSRGraph<WrappedNode<T>, WrappedNode<T>> for Graph<T> {
    fn build_directed(num_nodes: usize, edge_list: &EdgeList) -> Self {
        let mut graph = Graph::new(true);
        for v in 0..num_nodes {
            graph.add_vertex(v, None);
        }

        for (v, e, w) in edge_list {
            graph.add_edge(*v, *e, w, true)
        }
        graph
    }

    fn build_undirected(num_nodes: usize, edge_list: &EdgeList) -> Self {
        let mut graph = Graph::new(false);
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
        self.vertices.read().expect("Could not read").len()
    }

    fn num_edges(&self) -> usize {
        self.num_edges_undirected
    }

    fn num_edges_directed(&self) -> usize {
        self.num_edges_directed
    }

    fn out_degree(&self, v: NodeId) -> usize {
        if let Some(found) = self.vertices.read().expect("Could not read").get(&v) {
            found.read().expect("Could not read").out_edges.len()
        } else {
            0
        }
    }

    fn in_degree(&self, v: NodeId) -> usize {
        println!("Graph inversion is probably disabled... in in_degree()");
        if let Some(found) = self.vertices.read().expect("Could not read").get(&v) {
            found.read().expect("Could not read").in_edges.len()
        } else {
            panic!("Vertex not found");
        }
    }

    fn out_neigh(&self, v: NodeId) -> Range<WrappedNode<T>> {
        if let Some(vertex) = self.vertices.read().expect("Could not read").get(&v) {
            let mut edges = Vec::new();
            for edge in vertex.read().expect("Could not read").out_edges.values() {
                edges.push(WrappedNode::clone(&edge));
            }
            edges.sort_by(|a, b| a.as_node().cmp(&b.as_node()));
            Box::new(edges.into_iter())
        } else {
            panic!("Vertex not found");
        }
    }

    fn in_neigh(&self, v: NodeId) -> Range<WrappedNode<T>> {
        if let Some(vertex) = self.vertices.read().expect("Could not read").get(&v) {
            let mut edges = Vec::new();
            for edge in vertex.read().expect("Could not read").in_edges.values() {
                edges.push(WrappedNode::clone(&edge));
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

    fn vertices(&self) -> Range<WrappedNode<T>> {
        let mut edges = Vec::new();
        for edge in self.vertices.read().expect("Could not read").values() {
            edges.push(WrappedNode::clone(&edge));
        }
        edges.sort_by(|a, b| a.as_node().cmp(&b.as_node()));
        Box::new(edges.into_iter())
    }

    fn replace_out_edges(&self, v: NodeId, edges: Vec<WrappedNode<T>>) {
        if let Some(vertex) = self.vertices.read().expect("Could not read").get(&v) {
            let mut new_edges = HashMap::new();
            for e in edges {
                new_edges.insert(e.as_node(), e);
            }
            vertex.write().expect("Could not write").out_edges = new_edges;
        }
    }

    fn replace_in_edges(&self, v: NodeId, edges: Vec<WrappedNode<T>>) {
        if let Some(vertex) = self.vertices.read().expect("Could not read").get(&v) {
            let mut new_edges = HashMap::new();
            for e in edges {
                new_edges.insert(e.as_node(), e);
            }
            vertex.write().expect("Could not write").in_edges = new_edges;
        }
    }

    fn old_bfs(&self, v: NodeId) {
        self.bfs(v, None);
    }
}

impl<T> Graph<T> {
    pub fn new(directed: bool) -> Self {
        Graph {
            vertices: RwLock::new(HashMap::new()),
            num_nodes: 0,
            num_edges_directed: 0,
            num_edges_undirected: 0,
            directed,
        }
    }

    pub fn find_vertex(&self, vertex: usize) -> Option<Arc<RwLock<Node<T>>>> {
        self.vertices
            .read()
            .expect("Could not read")
            .get(&vertex)
            .map(|v| Arc::clone(v))
    }

    pub fn add_vertex(&self, node_id: usize, value: Option<T>) -> Arc<RwLock<Node<T>>> {
        let new_node = Node::new(node_id, value);
        self.vertices
            .write()
            .expect("Could not write")
            .entry(node_id)
            .or_insert(WrappedNode::from_node(new_node.clone(), &None));
        new_node
    }

    pub fn add_edge(&mut self, vertex: usize, edge: usize, weight: &Option<usize>, directed: bool) {
        if let (Some(vertex_node), Some(edge_node)) = (
            self.vertices.read().expect("Could not read").get(&vertex),
            self.vertices.read().expect("Could not read").get(&edge),
        ) {
            if !directed {
                self.num_edges_undirected += 1;
                Node::add_out_edge(&edge_node, &vertex_node, weight);
            } else {
                self.num_edges_directed += 1;
                Node::add_in_edge(&edge_node, &vertex_node, weight);
            }

            Node::add_out_edge(&vertex_node, &edge_node, weight);
        } else {
            panic!("Could not add edge, one or both of the nodes you are trying to connect does not exist");
        }
    }

    pub fn connect(
        &mut self,
        vertex_node: &Arc<RwLock<Node<T>>>,
        edge_node: &Arc<RwLock<Node<T>>>,
        weight: &Option<usize>,
        directed: bool,
    ) {
        if !directed {
            self.num_edges_undirected += 1;
            Node::add_out_edge(&edge_node, &vertex_node, weight);
        } else {
            self.num_edges_directed += 1;
            Node::add_in_edge(&edge_node, &vertex_node, weight);
        }

        Node::add_out_edge(&vertex_node, &edge_node, weight);
    }

    pub fn bfs(&self, start: usize, goal: Option<usize>) -> usize {
        let mut queue = VecDeque::new();
        let mut discovered = HashSet::new();

        let start = self.find_vertex(start).unwrap();
        discovered.insert(start.read().expect("Could not read").node_id);
        queue.push_back(Arc::clone(&start));

        while let Some(node) = queue.pop_front() {
            let readed_node = node.read().expect("Could not read");
            for edge in readed_node.out_edges.values() {
                let edge_node_id = edge.read().expect("Could not read").node_id;

                if goal == Some(edge_node_id) {
                    return discovered.len();
                }

                if !discovered.contains(&edge_node_id) {
                    discovered.insert(edge_node_id);
                    queue.push_back(Arc::clone(&edge));
                }
            }
        }

        discovered.len()
    }
}
