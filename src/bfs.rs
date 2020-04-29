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

#[derive(Clone)]
pub enum VisitStatus {
    Negative(NodeId),
    Positive(NodeId),
}

pub fn bu_step<V: AsNode, E: AsNode, G: CSRGraph<V, E>>(
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
pub fn td_step<V: AsNode, E: AsNode, G: CSRGraph<V, E>>(
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


pub fn init_parent<V: AsNode, E: AsNode, G: CSRGraph<V, E>>(graph: &G) -> Vec<VisitStatus> {
    let mut parent = vec![VisitStatus::Negative(1); graph.num_nodes()*10];
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
            parent.insert(v.as_node(), VisitStatus::Negative(graph.out_degree(v.as_node())));
        }
    }
    parent
}

fn queue_to_bitmap(queue: &SlidingQueue<NodeId>, bm: &mut BitVec) {
    for u in queue {
        bm.set(*u, true);
    }
}

fn bitmap_to_queue<V: AsNode, E: AsNode, G: CSRGraph<V, E>>(graph: &G, bm: &BitVec, queue: &mut SlidingQueue<NodeId>) {
    for n in 0..graph.num_nodes()*10 {
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

    let t_start = time::now_utc();

    let mut parent = init_parent(graph);
    let t_finish = time::now_utc();
    println!(
        "\tInit Parent: {} msec",
        (t_finish - t_start).num_milliseconds()
    );

    parent[source] = VisitStatus::Positive(source);
    let mut queue: SlidingQueue<NodeId> = SlidingQueue::with_capacity(graph.num_nodes());
    queue.push_back(source);
    queue.slide_window();

    let mut curr = BitVec::from_elem(graph.num_nodes()*10, false);
    let mut front = BitVec::from_elem(graph.num_nodes()*10, false);

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
                unsafe{std::ptr::swap(&mut front, &mut curr)};
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