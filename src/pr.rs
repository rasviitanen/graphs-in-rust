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

    let mut scores = HashMap::with_capacity(graph.num_nodes());
    let mut outgoing_contrib = HashMap::with_capacity(graph.num_nodes());

    for i in 0..max_iters {
        let mut error = 0.0;

        for n in 0..graph.num_nodes() {
            // FIXME: Should not be if/else, but division with zero¨
            // results in inf
            if graph.out_degree(n) != 0 {
                outgoing_contrib.insert(
                    n,
                    scores.get(&n).unwrap_or(&init_score) / graph.out_degree(n) as f64,
                );
            }
        }

        for u in graph.vertices() {
            let mut incoming_total = 0.0;

            for v in graph.in_neigh(u.as_node()) {
                incoming_total += outgoing_contrib.get(&v.as_node()).unwrap_or(&0.0);
            }

            let old_score = *scores.get(&u.as_node()).unwrap_or(&init_score);
            let new_score = base_score + K_DAMP * incoming_total;
            scores.insert(u.as_node(), new_score);
            error += f64::abs(new_score - old_score);
        }

        if error < epsilon {
            break;
        }
    }

    // assert!(verifier(graph, &scores, 0.0004));
    dbg!(&scores);
    scores.values().map(|x| *x).collect()
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

    dbg!(error);
    error < target_error
}
