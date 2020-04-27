use crate::types::*;
use crate::graph::CSRGraph;
use crate::slidingqueue::SlidingQueue;
use bit_vec::BitVec;
use rayon::iter::IndexedParallelIterator;
use rayon::iter::IntoParallelIterator;
use rayon::iter::IntoParallelRefMutIterator;
use rayon::iter::ParallelExtend;
use rayon::iter::ParallelIterator;
use rayon::iter::IntoParallelRefIterator;
use rayon::slice::ParallelSlice;

use std::sync::atomic::{AtomicUsize, Ordering};

pub fn bu_step<V: AsNode, E: AsNode, G: CSRGraph<V, E>>(
    graph: &G,
    parent: &mut Vec<Option<NodeId>>,
    front: &mut BitVec,
    next: &mut BitVec,
) -> usize {
    let awake_count = AtomicUsize::new(0);
    (0..graph.num_nodes()).into_iter().for_each(|u| {
        if parent[u].is_none() {
            for v in graph.in_neigh(u) {
                if front[v.as_node()] {
                    parent[v.as_node()] = Some(v.as_node());
                    awake_count.fetch_add(1, Ordering::SeqCst);
                    next.set(u, true);
                    break;
                }
            }
        }
    });

    awake_count.into_inner()
}

// FIXME: Make parallel
pub fn td_step<V: AsNode, E: AsNode, G: CSRGraph<V, E>>(
    graph: &G,
    parent: &mut Vec<Option<NodeId>>,
    queue: &mut SlidingQueue<NodeId>,
) -> usize {
    let mut scout_count = 0;
    let mut new_queue = SlidingQueue::new();
    for u in &*queue {
        graph.out_neigh(*u).into_iter().for_each(|v: E| {
            let curr_val = parent[v.as_node()];
            if curr_val.is_none() {
                parent[v.as_node()] = Some(*u);
                new_queue.push_back(v);  // FIXME: Not same as original code
                scout_count += 1; // FIXME: Not same as original code
            }
        });
    }

    new_queue.slide_window();

    for e in new_queue {
        queue.push_back(e.as_node());
    }

    queue.slide_window();


    scout_count
}


pub fn init_parent<V: AsNode, E: AsNode, G: CSRGraph<V, E>>(graph: &G) -> Vec<Option<NodeId>> {
    let mut parent = Vec::with_capacity(graph.num_nodes());
    parent.extend(
        (0..graph.num_nodes()).map(|n|
            if graph.out_degree(n) != 0 {
                Some(graph.out_degree(n)) // FIXME: Should be negated
            } else {
                None
            }
        )
    );
    parent
}

fn queue_to_bitmap(queue: &SlidingQueue<NodeId>, bm: &mut BitVec) {
    for u in queue {
        bm.set(*u, true);
    }
}

fn bitmap_to_queue<V: AsNode, E: AsNode, G: CSRGraph<V, E>>(graph: &G, bm: &BitVec, queue: &mut SlidingQueue<NodeId>) {
    for n in 0..graph.num_nodes() {
        if let Some(true) = bm.get(n) {
            queue.push_back(n);
        }
    }
    queue.slide_window();
}

pub fn do_bfs<V: AsNode, E: AsNode, G: CSRGraph<V, E>>(
    graph: &G,
    source: NodeId,
) {
    const ALPHA: usize = 15;
    const BETA: usize = 18;

    let mut parent = init_parent(graph);
    parent[source] = Some(source);
    let mut queue: SlidingQueue<NodeId> = SlidingQueue::with_capacity(graph.num_nodes());
    queue.push_back(source);
    queue.slide_window();

    let mut curr = BitVec::with_capacity(graph.num_nodes());
    let mut front = BitVec::with_capacity(graph.num_nodes());

    let mut edges_to_check = graph.num_edges_directed();
    let mut scout_count = graph.out_degree(source);

    while !queue.empty() {
        if scout_count > (edges_to_check / ALPHA) {
            queue_to_bitmap(&queue, &mut front);
            let awake_count = queue.size();
            let old_awake_count = 0;
            unimplemented!("BFS SHOULD NOT RUN HERE");
        } else {
            edges_to_check -= scout_count;
            scout_count = td_step(graph, &mut parent, &mut queue);
            queue.slide_window();
        }
    }
}