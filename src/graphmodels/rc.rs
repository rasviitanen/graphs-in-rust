use crate::graph::{CSRGraph, Range};
use crate::types::*;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::Rc;

#[derive(Clone)]
pub struct WrappedNode<T> {
    inner: Rc<RefCell<Node<T>>>,
}

impl<T> AsNode for WrappedNode<T> {
    fn as_node(&self) -> NodeId {
        self.inner.borrow().node_id
    }
}

impl<T> std::ops::Deref for WrappedNode<T> {
    type Target = Rc<RefCell<Node<T>>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> WrappedNode<T> {
    pub fn from_node(node: Rc<RefCell<Node<T>>>) -> Self {
        Self { inner: node }
    }
}

pub struct Node<T> {
    node_id: NodeId,
    value: Option<T>,
    in_edges: HashMap<usize, WrappedNode<T>>,
    out_edges: HashMap<usize, WrappedNode<T>>,
}

impl<T> Node<T> {
    pub fn new(node_id: NodeId, value: Option<T>) -> Rc<RefCell<Node<T>>> {
        let node = Node {
            node_id,
            value,
            in_edges: HashMap::new(),
            out_edges: HashMap::new(),
        };

        Rc::new(RefCell::new(node))
    }

    fn add_in_edge(this: &Rc<RefCell<Node<T>>>, edge: &Rc<RefCell<Node<T>>>)  -> bool {
        let node_id = edge.borrow().node_id;

        // Disable self-edges
        if this.borrow().node_id == node_id {
            return false;
        }

        this.borrow_mut()
            .in_edges
            .insert(node_id, WrappedNode::from_node(Rc::clone(edge))).is_none()
    }

    fn add_out_edge(this: &Rc<RefCell<Node<T>>>, edge: &Rc<RefCell<Node<T>>>) -> bool {
        let node_id = edge.borrow().node_id;

        // Disable self-edges
        if this.borrow().node_id == node_id {
            return false;
        }

        this.borrow_mut()
            .out_edges
            .insert(node_id, WrappedNode::from_node(Rc::clone(edge))).is_none()
    }
}

pub struct Graph<T> {
    vertices: RefCell<HashMap<usize, WrappedNode<T>>>,
    n_edges: Cell<usize>,
    directed: bool,
}

impl<T: Clone> CSRGraph<WrappedNode<T>, WrappedNode<T>> for Graph<T> {
    fn build_directed(num_nodes: usize, edge_list: &EdgeList) -> Self {
        let graph = Graph::new(true);
        for v in 0..num_nodes {
            graph.add_vertex(v, None);
        }

        for (v, e) in edge_list {
            graph.add_edge(*v, *e, true)
        }
        graph
    }

    fn build_undirected(num_nodes: usize, edge_list: &EdgeList) -> Self {
        let graph = Graph::new(false);
        println!("Building undirected, with {} nodes", num_nodes);
        for v in 0..num_nodes {
            graph.add_vertex(v, None);
        }
        for (v, e) in edge_list {
            graph.add_edge(*v, *e, false);
        }

        dbg!(&graph.n_edges);

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

    fn out_neigh(&self, v: NodeId) -> Range<WrappedNode<T>> {
        if let Some(vertex) = self.vertices.borrow().get(&v) {
            let mut edges = Vec::new();
            for edge in vertex.borrow().out_edges.values() {
                edges.push(WrappedNode::from_node(Rc::clone(edge)));
            }
            edges.sort_by(|a, b| a.as_node().cmp(&b.as_node()));
            Box::new(edges.into_iter())
        } else {
            panic!("Vertex not found");
        }
    }

    fn in_neigh(&self, v: NodeId) -> Range<WrappedNode<T>> {
        if let Some(vertex) = self.vertices.borrow().get(&v) {
            let mut edges = Vec::new();
            for edge in vertex.borrow().in_edges.values() {
                edges.push(WrappedNode::from_node(Rc::clone(edge)));
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
        for edge in self.vertices.borrow().values() {
            edges.push(WrappedNode::from_node(Rc::clone(edge)));
        }
        edges.sort_by(|a, b| a.as_node().cmp(&b.as_node()));
        Box::new(edges.into_iter())
    }

    fn replace_out_edges(&self, v: NodeId, edges: Vec<WrappedNode<T>>) {
        if let Some(vertex) = self.vertices.borrow().get(&v) {
            let mut new_edges = HashMap::new();
            for e in edges {
                new_edges.insert(e.as_node(), e);
            }
            vertex.borrow_mut().out_edges = new_edges;
        }
    }

    fn replace_in_edges(&self, v: NodeId, edges: Vec<WrappedNode<T>>) {
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

impl<T> Graph<T> {
    pub fn new(directed: bool) -> Self {
        Graph {
            vertices: RefCell::new(HashMap::new()),
            n_edges: Cell::new(0),
            directed,
        }
    }

    pub fn find_vertex(&self, vertex: usize) -> Option<Rc<RefCell<Node<T>>>> {
        self.vertices.borrow().get(&vertex).map(|v| Rc::clone(v))
    }

    pub fn add_vertex(&self, node_id: usize, value: Option<T>) -> Rc<RefCell<Node<T>>> {
        let new_node = Node::new(node_id, value);
        self.vertices
            .borrow_mut()
            .entry(node_id)
            .or_insert(WrappedNode::from_node(new_node.clone()));
        new_node
    }

    pub fn add_edge(&self, vertex: usize, edge: usize, directed: bool) {
        if let (Some(vertex_node), Some(edge_node)) = (
            self.vertices.borrow().get(&vertex),
            self.vertices.borrow().get(&edge),
        ) {
            if !directed {
                Node::add_out_edge(&edge_node, &vertex_node);
            } else {
                Node::add_in_edge(&edge_node, &vertex_node);
            }

            if Node::add_out_edge(&vertex_node, &edge_node) {
                self.n_edges.update(|x| x + 1);
            }
        } else {
            dbg!(vertex, edge, self.vertices.borrow().len());
            panic!("Could not add edge, one or both of the nodes you are trying to connect does not exist");
        }
    }

    pub fn connect(
        &self,
        vertex_node: &Rc<RefCell<Node<T>>>,
        edge_node: &Rc<RefCell<Node<T>>>,
        directed: bool,
    ) {
        if !directed {
            Node::add_out_edge(&edge_node, &vertex_node);
        } else {
            Node::add_in_edge(&edge_node, &vertex_node);
        }

        if Node::add_out_edge(&vertex_node, &edge_node) {
            self.n_edges.update(|x| x + 1);
        }
    }

    pub fn bfs(&self, start: usize, goal: Option<usize>) -> usize {
        let mut queue = VecDeque::new();
        let mut discovered = HashSet::new();

        let start = self.find_vertex(start).unwrap();
        discovered.insert(start.borrow().node_id);
        queue.push_back(Rc::clone(&start));

        while let Some(node) = queue.pop_front() {
            let locked_node = node.borrow();
            println!("Processing: {}", node.borrow().node_id);
            for edge in locked_node.out_edges.values() {
                let edge_node_id = edge.borrow().node_id;

                if goal == Some(edge_node_id) {
                    return discovered.len();
                }

                if !discovered.contains(&edge_node_id) {
                    discovered.insert(edge_node_id);
                    queue.push_back(Rc::clone(&edge));
                }
            }
        }

        discovered.len()
    }
}
