//! Will count the number of triangles (cliques of size 3)
//!
//! Requires input graph:
//!   - to be undirected
//!   - no duplicate edges (or else will be counted as multiple triangles)
//!   - neighborhoods are sorted by vertex identifiers
//!
//! Other than symmetrizing, the rest of the requirements are done by SquishCSR
//! during graph building.
//!
//! This implementation reduces the search space by counting each triangle only
//! once. A naive implementation will count the same triangle six times because
//! each of the three vertices `(u, v, w)` will count it in both ways. To count
//! a triangle only once, this implementation only counts a triangle if `u > v > w`.
//! Once the remaining unexamined neighbors identifiers get too big, it can break
//! out of the loop, but this requires that the neighbors to be sorted.
//! Another optimization this implementation has is to relabel the vertices by
//! degree. This is beneficial if the average degree is high enough and if the
//! degree distribution is sufficiently non-uniform. To decide whether or not
//! to relabel the graph, we use the heuristic in WorthRelabelling.

use crate::graph::CSRGraph;
use crate::types::*;
use rayon::iter::IndexedParallelIterator;
use rayon::iter::IntoParallelIterator;
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::IntoParallelRefMutIterator;
use rayon::iter::ParallelExtend;
use rayon::iter::ParallelIterator;
use rayon::slice::ParallelSlice;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Has been manually verified,
/// Only works on undirected, with sorted nodes
fn ordered_count<'a, V: AsNode, E: AsNode, G: CSRGraph<V, E>>(graph: &G) -> usize {
    let mut total = 0;
    for u in 0..graph.num_nodes() {
        for v in graph.out_neigh(u) {
            if v.as_node() > u {
                break;
            }

            let it: Vec<_> = graph.out_neigh(u).collect();
            let mut idx = 0;
            for w in graph.out_neigh(v.as_node()) {
                if w.as_node() > v.as_node() {
                    break;
                }

                while it[idx].as_node() < w.as_node() {
                    idx += 1;
                }

                if w.as_node() == it[idx].as_node() {
                    total += 1;
                }

                idx = 0;
            }
        }
    }

    total
}

fn ordered_count_mt<'a, V: AsNode, E: AsNode, G: Send + Sync + CSRGraph<V, E>>(graph: &G) -> usize {
    (0..graph.num_nodes())
        .into_par_iter()
        .map(|u| {
            let mut count = 0;
            for v in graph.out_neigh(u) {
                if v.as_node() > u {
                    break;
                }

                let it: Vec<_> = graph.out_neigh(u).collect();
                let mut idx = 0;
                for w in graph.out_neigh(v.as_node()) {
                    if w.as_node() > v.as_node() {
                        break;
                    }

                    while it[idx].as_node() < w.as_node() {
                        idx += 1;
                    }

                    if w.as_node() == it[idx].as_node() {
                        count += 1;
                    }

                    idx = 0;
                }
            }
            count
        })
        .sum()
}

fn verifier<'a, V: AsNode, E: AsNode, G: CSRGraph<V, E>>(graph: &G, test_total: usize) -> bool {
    let mut total = 0;

    for u in graph.vertices() {
        for v in graph.out_neigh(u.as_node()) {
            let v_edges: std::collections::HashSet<NodeId> =
                graph.out_neigh(v.as_node()).map(|x| x.as_node()).collect();
            let u_edges: std::collections::HashSet<NodeId> =
                graph.out_neigh(u.as_node()).map(|x| x.as_node()).collect();

            let intersection = u_edges.intersection(&v_edges);

            total += intersection.count();
        }
    }

    total = total / 6; // Each triangle was counted 6 times

    total == test_total
}

fn worth_relabelling<'a, V: AsNode, E: AsNode, G: CSRGraph<V, E>>(graph: &G) -> bool {
    // FIXME: Implement this
    false
}

pub fn hybrid<'a, V: AsNode, E: AsNode, G: CSRGraph<V, E>>(graph: &G) {
    if worth_relabelling(graph) {
        unimplemented!("Relabeling is not supported");
    } else {
        let res = ordered_count(graph);
        // verifier(graph, res);
    }
}
pub fn hybrid_mt<'a, V: AsNode, E: AsNode, G: Sync + Send + CSRGraph<V, E>>(graph: &G) {
    if worth_relabelling(graph) {
        unimplemented!("Relabeling is not supported");
    } else {
        let res = ordered_count_mt(graph);
        // verifier(graph, res);
    }
}
