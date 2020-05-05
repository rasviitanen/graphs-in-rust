//! Will return pagerank scores for all vertices once total change < epsilon
//! This PR implementation uses the traditional iterative approach. This is done
//! to ease comparisons to other implementations (often use same algorithm), but
//! it is not necesarily the fastest way to implement it. It does perform the
//! updates in the pull direction to remove the need for atomics.

use crate::graph::CSRGraph;
use crate::types::*;
use std::collections::HashMap;

type Score = f64;
const K_DAMP: f64 = 0.85;

/// Has been manually verified,
/// Only works on undirected, with sorted nodes
pub fn page_rank_pull<V: AsNode, E: AsNode, G: CSRGraph<V, E>>(
    graph: &G,
    max_iters: usize,
    epsilon: Option<f64>,
) -> Vec<Score> {
    let epsilon = epsilon.unwrap_or(0.0);

    let init_score = 1.0 / graph.num_nodes() as f64;
    let base_score = (1.0 - K_DAMP) / graph.num_nodes() as f64;

    let mut scores = vec![init_score; graph.num_nodes()];
    let mut outgoing_contrib = vec![0.0; graph.num_nodes()];

    for i in 0..max_iters {
        let mut error = 0.0;

        for n in 0..graph.num_nodes() {
            outgoing_contrib[n] = scores[n] / graph.out_degree(n) as f64;
        }

        for u in 0..graph.num_nodes() {
            let mut incoming_total = 0.0;

            for v in graph.in_neigh(u) {
                incoming_total += outgoing_contrib[v.as_node()];
            }

            let old_score = scores[u];
            scores[u] = base_score + K_DAMP * incoming_total;
            error += f64::abs(scores[u] - old_score);
        }

        if error < epsilon {
            break;
        }
    }

    assert!(verifier(graph, &scores, 0.0004));
    scores
}

pub fn verifier<V: AsNode, E: AsNode, G: CSRGraph<V, E>>(
    graph: &G,
    scores: &Vec<Score>,
    target_error: f64,
) -> bool {
    let base_score = (1.0 - K_DAMP) / graph.num_nodes() as f64;

    let mut incoming_sums = vec![0.0; graph.num_nodes()];
    let mut error = 0.0;

    for u in graph.vertices() {
        let outgoing_contrib = scores[u.as_node()] / graph.out_degree(u.as_node()) as f64;
        for v in graph.out_neigh(u.as_node()) {
            incoming_sums[v.as_node()] += outgoing_contrib;
        }
    }

    for n in graph.vertices() {
        error += f64::abs(base_score + K_DAMP * incoming_sums[n.as_node()] - scores[n.as_node()]);
        incoming_sums[n.as_node()] = 0.0;
    }

    error < target_error
}
