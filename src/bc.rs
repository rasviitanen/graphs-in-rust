//! Will return array of approx betweenness centrality scores for each vertex
//!
//! This BC implementation makes use of the Brandes \[1\] algorithm with
//! implementation optimizations from Madduri et al. \[2\]. It is only an approximate
//! because it does not compute the paths from every start vertex, but only a small
//! subset of them. Additionally, the scores are normalized to the range `[0,1]`.
//! As an optimization to save memory, this implementation uses a Bitmap to hold
//! succ (list of successors) found during the BFS phase that are used in the back-
//! propagation phase.
//!
//! ## Sources
//! \[1\] Ulrik Brandes. "A faster algorithm for betweenness centrality." Journal of
//!     Mathematical Sociology, 25(2):163–177, 2001.
//! \[2\] Kamesh Madduri, David Ediger, Karl Jiang, David A Bader, and Daniel
//!     Chavarria-Miranda. "A faster parallel algorithm and efficient multithreaded
//!     implementations for evaluating betweenness centrality on massive datasets."
//!     International Symposium on Parallel & Distributed Processing (IPDPS), 2009.
//!
//! ## Warning
//! This statement is false:
//! As an optimization to save memory, this implementation uses a Bitmap to hold
//! succ (list of successors).
//!
//! We use a hashset instead, as the pointer-magic in C++ is not easily
//! ported to rust

use crate::graph::CSRGraph;
use crate::slidingqueue::SlidingQueue;
use crate::benchmark::SourcePicker;
use crate::types::*;
use bit_vec::BitVec;
use rayon::iter::IndexedParallelIterator;
use rayon::iter::IntoParallelIterator;
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::IntoParallelRefMutIterator;
use rayon::iter::ParallelExtend;
use rayon::iter::ParallelIterator;
use rayon::slice::ParallelSlice;
use std::collections::HashSet;

type Score = f64;
type Count = f64;
type Bitmap = Vec<bool>;

fn pbfs<V: AsNode, E: AsNode, G: CSRGraph<V, E>>(
    graph: &G,
    source: NodeId,
    path_counts: &mut Vec<Count>,
    succ: &mut HashSet<(usize, usize)>,
    depth_index: &mut Vec<Vec<NodeId>>,
    queue: &mut SlidingQueue<NodeId>,
) {
    let mut depths: Vec<Option<NodeId>> = vec![None; graph.num_nodes()];
    depths[source] = Some(0);
    path_counts[source] = 1.0;
    queue.push_back(source);
    let queue_clone: Vec<NodeId> = queue.into_iter().map(|x| *x).collect();
    depth_index.push(queue_clone);
    queue.slide_window();

    {
        let mut depth = 0;
        let mut lqueue = SlidingQueue::new();
        while !queue.empty() {
            depth += 1;

            for u in queue.into_iter() {
                for v in graph.out_neigh(*u) {
                    let v = v.as_node();
                    if depths[v].is_none() {
                        depths[v] = Some(depth);
                        lqueue.push_back(v);
                    }

                    if depths[v] == Some(depth) {
                        succ.insert((*u, v));
                        path_counts[v] += path_counts[*u];
                    }
                }
            }

            lqueue.slide_window();

            for e in &lqueue {
                queue.push_back(*e);
            }
    
            let queue_clone: Vec<NodeId> = queue.into_iter().map(|x| *x).collect();
            depth_index.push(queue_clone);
            queue.slide_window();
        }
    }
    let queue_clone: Vec<NodeId> = queue.into_iter().map(|x| *x).collect();
    depth_index.push(queue_clone)
}

pub fn brandes<V: AsNode, E: AsNode, G: CSRGraph<V, E>>(
    graph: &G,
    sp: &mut SourcePicker<V, E, G>,
    num_iters: NodeId,
) -> Vec<Score> {
    let mut scores = vec![0.0; graph.num_nodes()];
    let mut path_counts = vec![0.0; graph.num_nodes()];
    let mut succ = HashSet::new();
    let mut depth_index: Vec<Vec<NodeId>> = Vec::new();
    let mut queue = SlidingQueue::with_capacity(graph.num_nodes());

    for n in 0..num_iters {
        let source = sp.pick_next();
        for e in path_counts.iter_mut() {
            *e = 0.0;
        }
        depth_index.clear();
        queue.reset();
        succ.clear();

        pbfs(graph, source, &mut path_counts, &mut succ, &mut depth_index, &mut queue);

        let mut deltas = vec![0.0; graph.num_nodes()];

        for d in (0..=depth_index.len()-2).rev() {
            let end_check = depth_index[d+1].get(0);
            for u in &depth_index[d] {
                if let Some(other_start) = end_check {
                    if u == other_start {
                        break;
                    }
                }

                let mut delta_u = 0.0;
                for v in graph.out_neigh(*u) {
                    let v = v.as_node();
                    if succ.get(&(*u, v)).is_some() {
                        delta_u += (path_counts[*u] / path_counts[v]) * (1.0 + deltas[v]);
                    }
                }

                deltas[*u] = delta_u;
                scores[*u] += delta_u;
            }
        }
    }

    let mut biggest_score = 0.0;

    for n in 0..graph.num_nodes() {
        biggest_score = f64::max(biggest_score, scores[n]);
    }

    for n in 0..graph.num_nodes() {
        scores[n] = scores[n] / biggest_score;
    }

    scores
}