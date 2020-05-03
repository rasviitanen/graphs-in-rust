//! Will return comp array labelling each vertex with a connected component ID
//! This CC implementation makes use of the Afforest subgraph sampling algorithm \[1\],
//! which restructures and extends the Shiloach-Vishkin algorithm \[2\].
//!
//! ## Sources
//! \[1\] Michael Sutton, Tal Ben-Nun, and Amnon Barak. "Optimizing Parallel
//!     Graph Connectivity Computation via Subgraph Sampling" Symposium on
//!     Parallel and Distributed Processing, IPDPS 2018.
//! \[2\] Yossi Shiloach and Uzi Vishkin. "An o(logn) parallel connectivity algorithm"
//!     Journal of Algorithms, 3(1):57–67, 1982.

use crate::graph::CSRGraph;
use crate::types::*;
use bit_vec::BitVec;
use rayon::iter::IndexedParallelIterator;
use rayon::iter::IntoParallelIterator;
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::IntoParallelRefMutIterator;
use rayon::iter::ParallelExtend;
use rayon::iter::ParallelIterator;
use rayon::slice::ParallelSlice;
use std::collections::HashMap;
use rand::prelude::*;
use std::collections::VecDeque;

fn link(u: NodeId, v: NodeId, comp: &mut Vec<NodeId>) {
    let mut p1 = comp[u];
    let mut p2 = comp[v];

    while p1 != p2 {
        let (high, low) = if p1 > p2 {
            (p1, p2)
        } else {
            (p2, p1)
        };

        let p_high = comp[high];

        if p_high == low {
            break;
        }

        if p_high == high && comp[high] == high {
            // FIXME: They use atomic CAS here, but it should not be needed
            // for single-threaded execution.
            comp[high] = low;
            break;
        }

        p1 = comp[comp[high]];
        p2 = comp[low];
    }
}

// FIXME: Make parallel
fn compress<V: AsNode, E: AsNode, G: CSRGraph<V, E>>(
    graph: &G,
    comp: &mut Vec<NodeId>,
) {
    (0..graph.num_nodes()).into_iter().for_each(|n| {
        while comp[n] != comp[comp[n]] {
            comp[n] = comp[comp[n]];
        }
    });
}

fn sample_frequent_element(
    comp: &Vec<NodeId>,
    num_samples: Option<usize>,
) -> NodeId {
    // Sample elements from `comp`
    let num_samples = num_samples.unwrap_or(1024);
    let mut sample_counts = HashMap::with_capacity(32);

    let mut rng = rand::thread_rng();
    let uniform_distribution = rand::distributions::Uniform::from(0..comp.len());

    for i in 0..num_samples {
        let n = uniform_distribution.sample(&mut rng);
        *sample_counts.entry(comp[n]).or_insert(0) += 1;
    }

    // Find most frequent element in sampless
    let most_frequent = sample_counts
        .iter()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .expect("Failed to calculate most frequent");


    let frac_of_graph: f64 = *most_frequent.1 as f64 / num_samples as f64;

    println!(
        "Skipping largest intermediate component
        (ID: {}, approx. {}% of the graph.)",
        most_frequent.0,
        frac_of_graph*100.0,
    );

    *most_frequent.0
}

pub fn afforest<V: AsNode, E: AsNode, G: CSRGraph<V, E>>(
    graph: &G,
    neighbor_rounds: Option<usize>,
) -> Vec<NodeId> {
    let neighbor_rounds = neighbor_rounds.unwrap_or(2);
    // FIXME: Make parallel
    let mut comp: Vec<NodeId> = (0..graph.num_nodes()).into_iter().collect();

    for r in 0..neighbor_rounds {
        for u in 0..graph.num_nodes() {
            for v in graph.out_neigh(u).skip(r) {
                link(u, v.as_node(), &mut comp);
                break;
            }
        }
        compress(graph, &mut comp);
    }

    let c = sample_frequent_element(&comp, None);

    if !graph.directed() {
        for u in 0..graph.num_nodes() {
            // Skip processing nodes in the largest component
            if comp[u] == c {
                continue;
            }
            // Skip oveer part of the neighborhood (determined by neighor_rounds)
            for v in graph.out_neigh(u).skip(neighbor_rounds) {
                link(u, v.as_node(), &mut comp);
            }
        };
    } else {
        for u in 0..graph.num_nodes() {
            if comp[u] == c {
                continue;
            }

            for v in graph.out_neigh(u).skip(neighbor_rounds) {
                link(u, v.as_node(), &mut comp);
            }

            for v in graph.in_neigh(u).skip(neighbor_rounds) {
                println!("Link({}, {})", u, v.as_node());
                link(u, v.as_node(), &mut comp);
            }
        }
    }

    // Finally, `compress` for final convergence
    compress(graph, &mut comp);
    dbg!(verifier(graph, &comp));
    dbg!(&comp);

    comp
}

fn verifier<V: AsNode, E: AsNode, G: CSRGraph<V, E>>(
    graph: &G,
    comp: &Vec<NodeId>,
) -> bool {
    let mut label_to_source = HashMap::new();

    for n in graph.vertices() {
        label_to_source.insert(comp[n.as_node()], n.as_node());
    }

    let mut visited = vec![false; graph.num_nodes()];

    let mut frontier: VecDeque<NodeId> = VecDeque::with_capacity(graph.num_nodes());

    for label_source_pair in label_to_source {
        let curr_label = label_source_pair.0;
        let source = label_source_pair.1;

        frontier.clear();
        frontier.push_back(source);
        visited[source] = true;

        while let Some(u) = frontier.pop_front() {
            for v in graph.out_neigh(u) {
                if comp[v.as_node()] != curr_label {
                    return false;
                }
                if !visited[v.as_node()] {
                    visited[v.as_node()] = true;
                    frontier.push_back(v.as_node());
                }
            }

            if graph.directed() {
                for v in graph.in_neigh(u) {
                    if comp[v.as_node()] != curr_label {
                        return false;
                    }
                    if !visited[v.as_node()] {
                        visited[v.as_node()] = true;
                        frontier.push_back(v.as_node());
                    }
                }
            }
        }
    }

    for n in 0..graph.num_nodes() {
        if !visited[n] {
            return false;
        }
    }

    true
}