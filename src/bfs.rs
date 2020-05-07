//! Will return parent array for a BFS traversal from a source vertex
//!
//! This BFS implementation makes use of the Direction-Optimizing approach \[1\].
//! It uses the alpha and beta parameters to determine whether to switch search
//! directions. For representing the frontier, it uses a SlidingQueue for the
//! top-down approach and a Bitmap for the bottom-up approach. To reduce
//! false-sharing for the top-down approach, thread-local QueueBuffer's are used.
//! To save time computing the number of edges exiting the frontier, this
//! implementation precomputes the degrees in bulk at the beginning by storing
//! them in parent array as negative numbers. Thus the encoding of parent is:
//!
//! * `parent[x] < 0 implies x is unvisited and parent[x] = -out_degree(x)`
//! * `parent[x] >= 0 implies x been visited`
//!
//! ## Sources
//! > \[1\] Scott Beamer, Krste Asanović, and David Patterson. "Direction-Optimizing
//!   Breadth-First Search." International Conference on High Performance
//!   Computing, Networking, Storage and Analysis (SC), Salt Lake City, Utah,
//!   November 2012.

use crate::graph::CSRGraph;
use crate::slidingqueue::SlidingQueue;
use crate::types::*;
use bit_vec::BitVec;
use rayon::iter::IndexedParallelIterator;
use rayon::iter::IntoParallelIterator;
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::IntoParallelRefMutIterator;
use rayon::iter::ParallelExtend;
use rayon::iter::ParallelIterator;
use rayon::slice::ParallelSlice;

use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Clone)]
pub enum VisitStatus {
    Negative(NodeId),
    Positive(NodeId),
}

pub fn bu_step<'a, V: AsNode, E: AsNode, G: CSRGraph<V, E>>(
    graph: &G,
    parent: &mut Vec<VisitStatus>,
    front: &mut BitVec,
    next: &mut BitVec,
) -> usize {
    let awake_count = AtomicUsize::new(0);
    graph.vertices().for_each(|u| {
        if let VisitStatus::Negative(curr_val) = parent[u.as_node()] {
            if curr_val != 0 {
                for v in graph.in_neigh(u.as_node()) {
                    if front[v.as_node()] {
                        parent[v.as_node()] = VisitStatus::Positive(v.as_node());
                        awake_count.fetch_add(1, Ordering::SeqCst);
                        next.set(u.as_node(), true);
                        break;
                    }
                }
            }
        }
    });

    awake_count.into_inner()
}

// FIXME: Make parallel
pub fn td_step<'a, V: AsNode, E: AsNode, G: CSRGraph<V, E>>(
    graph: &G,
    parent: &mut Vec<VisitStatus>,
    queue: &mut SlidingQueue<NodeId>,
) -> usize {
    let mut scout_count = 0;
    let mut new_queue = SlidingQueue::new();

    for u in &*queue {
        graph.out_neigh(*u).into_iter().for_each(|v: E| {
            if let VisitStatus::Negative(curr_val) = parent[v.as_node()] {
                if curr_val != 0 {
                    parent.insert(v.as_node(), VisitStatus::Positive(*u));
                    new_queue.push_back(v.as_node());
                    scout_count += curr_val;
                }
            }
        });
    }

    new_queue.slide_window();

    for e in new_queue {
        queue.push_back(e);
    }

    scout_count
}

pub fn init_parent<'a, V: AsNode, E: AsNode, G: CSRGraph<V, E>>(graph: &G) -> Vec<VisitStatus> {
    let mut parent = vec![VisitStatus::Negative(1); graph.num_nodes() * 10];
    // parent.extend(
    //     (0..graph.vertices().map(|n|
    //         if graph.out_degree(n.as_node()) != 0 {
    //             VisitStatus::Negative(graph.out_degree(n))
    //         } else {
    //             VisitStatus::Negative(1)
    //         }
    //     )
    // );
    for v in graph.vertices() {
        if graph.out_degree(v.as_node()) != 0 {
            parent.insert(
                v.as_node(),
                VisitStatus::Negative(graph.out_degree(v.as_node())),
            );
        }
    }
    parent
}

fn queue_to_bitmap(queue: &SlidingQueue<NodeId>, bm: &mut BitVec) {
    for u in queue {
        bm.set(*u, true);
    }
}

fn bitmap_to_queue<'a, V: AsNode, E: AsNode, G: CSRGraph<V, E>>(
    graph: &G,
    bm: &BitVec,
    queue: &mut SlidingQueue<NodeId>,
) {
    for n in 0..graph.num_nodes() * 10 {
        if let Some(true) = bm.get(n) {
            queue.push_back(n);
        }
    }
    queue.slide_window();
}

pub fn do_bfs<'a, V: AsNode, E: AsNode, G: CSRGraph<V, E>>(graph: &G, source: NodeId) {
    const ALPHA: usize = 15;
    const BETA: usize = 18;

    // let timer = crate::timer::ScopedTimer::new("Init Parent");
    let mut parent = init_parent(graph);
    // drop(timer);

    parent[source] = VisitStatus::Positive(source);
    let mut queue: SlidingQueue<NodeId> = SlidingQueue::with_capacity(graph.num_nodes());
    queue.push_back(source);
    queue.slide_window();

    let mut curr = BitVec::from_elem(graph.num_nodes() * 10, false);
    let mut front = BitVec::from_elem(graph.num_nodes() * 10, false);

    let mut edges_to_check = graph.num_edges_directed();
    let mut scout_count = graph.out_degree(source);

    while !queue.empty() {
        if scout_count > (edges_to_check / ALPHA) {
            queue_to_bitmap(&queue, &mut front);

            let mut awake_count = queue.size();
            queue.slide_window();

            loop {
                let old_awake_count = awake_count;
                awake_count = bu_step(graph, &mut parent, &mut front, &mut curr);
                unsafe { std::ptr::swap(&mut front, &mut curr) };
                if (awake_count >= old_awake_count) || (awake_count > graph.num_nodes() / BETA) {
                    break;
                }
            }

            bitmap_to_queue(graph, &mut front, &mut queue);
            scout_count = 1;
        } else {
            edges_to_check -= scout_count;
            scout_count = td_step(graph, &mut parent, &mut queue);
            queue.slide_window();
        }
    }
}
