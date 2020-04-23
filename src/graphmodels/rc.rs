use crate::graph::{CSRGraph, Range};
use crate::types::*;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::Rc;

pub struct Node<T> {
    key: usize,
    value: Option<T>,
    adj_list: HashMap<usize, Rc<RefCell<Self>>>,
}

impl<T> Node<T> {
    pub fn new(key: usize, value: Option<T>) -> Rc<RefCell<Node<T>>> {
        let node = Node {
            key,
            value,
            adj_list: HashMap::new(),
        };

        Rc::new(RefCell::new(node))
    }

    fn add_edge(this: &Rc<RefCell<Node<T>>>, edge: &Rc<RefCell<Node<T>>>) {
        let key = edge.borrow().key;
        this.borrow_mut().adj_list.insert(key, Rc::clone(edge));
    }
}

pub struct Graph<T> {
    vertices: RefCell<HashMap<usize, Rc<RefCell<Node<T>>>>>,
}



impl<T> CSRGraph for Graph<T> {
    fn directed(&self) -> bool {
        unimplemented!();
    }

    fn num_nodes(&self) -> usize {
        unimplemented!();
    }
    
    fn num_edges(&self) -> usize {
        unimplemented!();
    }

    fn num_edges_directed(&self) -> usize {
        unimplemented!();
    }

    fn out_degree(&self, v: NodeId) -> usize {
        unimplemented!();
    }

    fn in_degree(&self, v: NodeId) -> usize {
        unimplemented!();
    }

    fn print_stats(&self) {
        unimplemented!();
    }

    fn vertices<V>(&self) -> Range<V> {
        unimplemented!();
    }
}

impl<T> Graph<T> {
    pub fn new() -> Self {
        Graph {
            vertices: RefCell::new(HashMap::new()),
        }
    }

    pub fn find_vertex(&self, vertex: usize) -> Option<Rc<RefCell<Node<T>>>> {
        self.vertices.borrow().get(&vertex).map(|v| Rc::clone(v))
    }

    pub fn add_vertex(&self, key: usize, value: Option<T>) -> Rc<RefCell<Node<T>>> {
        let new_node = Node::new(key, value);
        self.vertices
            .borrow_mut()
            .entry(key)
            .or_insert(new_node.clone());
        new_node
    }

    pub fn add_edge(&self, vertex: usize, edge: usize) {
        if let (Some(vertex_node), Some(edge_node)) = (
            self.vertices.borrow().get(&vertex),
            self.vertices.borrow().get(&edge),
        ) {
            Node::add_edge(&vertex_node, &edge_node)
        } else {
            panic!("Could not add edge, one or both of the nodes you are trying to connect does not exist");
        }
    }

    pub fn connect(vertex_node: &Rc<RefCell<Node<T>>>, edge_node: &Rc<RefCell<Node<T>>>) {
        Node::add_edge(&vertex_node, &edge_node)
    }

    pub fn bfs(&self, start: usize, goal: Option<usize>) -> usize {
        let mut queue = VecDeque::new();
        let mut discovered = HashSet::new();

        let start = self.find_vertex(start).unwrap();
        discovered.insert(start.borrow().key);
        queue.push_back(Rc::clone(&start));

        while let Some(node) = queue.pop_front() {
            let locked_node = node.borrow();
            for edge in locked_node.adj_list.values() {
                let edge_node_id = edge.borrow().key;

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
