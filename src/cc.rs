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
use rand::prelude::*;
use rayon::iter::IndexedParallelIterator;
use rayon::iter::IntoParallelIterator;
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::IntoParallelRefMutIterator;
use rayon::iter::ParallelExtend;
use rayon::iter::ParallelIterator;
use rayon::slice::ParallelSlice;
use std::collections::HashMap;
use std::collections::VecDeque;

fn link(u: NodeId, v: NodeId, comp: &mut Vec<NodeId>) {
    let mut p1 = comp[u];
    let mut p2 = comp[v];

    while p1 != p2 {
        let (high, low) = if p1 > p2 { (p1, p2) } else { (p2, p1) };

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

fn compress<'a, V: AsNode, E: AsNode, G: CSRGraph<V, E>>(graph: &G, comp: &mut Vec<NodeId>) {
    (0..graph.num_nodes()).into_iter().for_each(|n| {
        while comp[n] != comp[comp[n]] {
            comp[n] = comp[comp[n]];
        }
    });
}

#[derive(Copy, Clone)]
struct UnsafeBox(*mut usize);

unsafe impl Send for UnsafeBox {}
unsafe impl Sync for UnsafeBox {}

fn compress_mt<'a, V: AsNode, E: AsNode, G: Send + Sync + CSRGraph<V, E>>(
    graph: &G,
    comp: &mut Vec<NodeId>,
) {
    let ptr: UnsafeBox = UnsafeBox(comp.as_mut_ptr());
    (0..graph.num_nodes()).into_par_iter().for_each(|n| {
        // This unsafety makes me nervous
        unsafe {
            let first = ptr.0.wrapping_offset(n as isize);
            let second = ptr.0.wrapping_offset(*first as isize);
            while *first != *second {
                *first = *second;
            }
        }
    });
}

fn sample_frequent_element(comp: &Vec<NodeId>, num_samples: Option<usize>) -> NodeId {
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

    *most_frequent.0
}

pub fn afforest<'a, V: AsNode, E: AsNode, G: CSRGraph<V, E>>(
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
        }
    } else {
        for u in 0..graph.num_nodes() {
            if comp[u] == c {
                continue;
            }

            for v in graph.out_neigh(u).skip(neighbor_rounds) {
                link(u, v.as_node(), &mut comp);
            }

            for v in graph.in_neigh(u).skip(neighbor_rounds) {
                link(u, v.as_node(), &mut comp);
            }
        }
    }

    // Finally, `compress` for final convergence
    compress(graph, &mut comp);
    comp
}

pub fn afforest_mt<'a, V: AsNode, E: AsNode, G: Send + Sync + CSRGraph<V, E>>(
    graph: &G,
    neighbor_rounds: Option<usize>,
) -> Vec<NodeId> {
    let neighbor_rounds = neighbor_rounds.unwrap_or(2);
    // FIXME: Make parallel
    let mut comp: Vec<NodeId> = (0..graph.num_nodes()).into_par_iter().collect();
    let comp_ptr: UnsafeBox = UnsafeBox(comp.as_mut_ptr());

    for r in 0..neighbor_rounds {
        (0..graph.num_nodes()).into_par_iter().for_each(|u| {
            for v in graph.out_neigh(u).skip(r) {
                let mut p1 = comp_ptr.0.wrapping_offset(u as isize);
                let mut p2 = comp_ptr.0.wrapping_offset(v.as_node() as isize);

                while unsafe { *p1 != *p2 } {
                    let (high, low) = unsafe {
                        if *p1 > *p2 {
                            (*p1, *p2)
                        } else {
                            (*p2, *p1)
                        }
                    };

                    let p_high = comp_ptr.0.wrapping_offset(high as isize);

                    if unsafe { *p_high == low } {
                        break;
                    }

                    if unsafe {
                        (*p_high == high) && (*comp_ptr.0.wrapping_offset(high as isize) == high)
                    } {
                        // FIXME: They use atomic CAS here, but it should not be needed
                        // for single-threaded execution.
                        unsafe {
                            *comp_ptr.0.wrapping_offset(high as isize) = low;
                        }
                        break;
                    }

                    unsafe {
                        p1 = comp_ptr
                            .0
                            .wrapping_offset((*comp_ptr.0.wrapping_offset(high as isize)) as isize);
                        p2 = comp_ptr.0.wrapping_offset(low as isize);
                    }
                }

                break;
            }
        });
        compress_mt(graph, &mut comp);
    }

    let c = sample_frequent_element(&comp, None);

    if !graph.directed() {
        (0..graph.num_nodes()).into_par_iter().for_each(|u| {
            if unsafe { *comp_ptr.0.wrapping_offset(u as isize) != c } {
                for v in graph.out_neigh(u).skip(neighbor_rounds) {
                    let mut p1 = comp_ptr.0.wrapping_offset(u as isize);
                    let mut p2 = comp_ptr.0.wrapping_offset(v.as_node() as isize);

                    while unsafe { *p1 != *p2 } {
                        let (high, low) = unsafe {
                            if *p1 > *p2 {
                                (*p1, *p2)
                            } else {
                                (*p2, *p1)
                            }
                        };

                        let p_high = comp_ptr.0.wrapping_offset(high as isize);

                        if unsafe { *p_high == low } {
                            break;
                        }

                        if unsafe {
                            (*p_high == high)
                                && (*comp_ptr.0.wrapping_offset(high as isize) == high)
                        } {
                            // FIXME: They use atomic CAS here, but it should not be needed
                            // for single-threaded execution.
                            unsafe {
                                *comp_ptr.0.wrapping_offset(high as isize) = low;
                            }
                            break;
                        }

                        unsafe {
                            p1 = comp_ptr.0.wrapping_offset(
                                (*comp_ptr.0.wrapping_offset(high as isize)) as isize,
                            );
                            p2 = comp_ptr.0.wrapping_offset(low as isize);
                        }
                    }

                    break;
                }
            }
        });
    } else {
        (0..graph.num_nodes()).into_par_iter().for_each(|u| {
            if unsafe { *comp_ptr.0.wrapping_offset(u as isize) != c } {
                for v in graph.out_neigh(u).skip(neighbor_rounds) {
                    for v in graph.out_neigh(u).skip(neighbor_rounds) {
                        let mut p1 = comp_ptr.0.wrapping_offset(u as isize);
                        let mut p2 = comp_ptr.0.wrapping_offset(v.as_node() as isize);

                        while unsafe { *p1 != *p2 } {
                            let (high, low) = unsafe {
                                if *p1 > *p2 {
                                    (*p1, *p2)
                                } else {
                                    (*p2, *p1)
                                }
                            };

                            let p_high = comp_ptr.0.wrapping_offset(high as isize);

                            if unsafe { *p_high == low } {
                                break;
                            }

                            if unsafe {
                                (*p_high == high)
                                    && (*comp_ptr.0.wrapping_offset(high as isize) == high)
                            } {
                                // FIXME: They use atomic CAS here, but it should not be needed
                                // for single-threaded execution.
                                unsafe {
                                    *comp_ptr.0.wrapping_offset(high as isize) = low;
                                }
                                break;
                            }

                            unsafe {
                                p1 = comp_ptr.0.wrapping_offset(
                                    (*comp_ptr.0.wrapping_offset(high as isize)) as isize,
                                );
                                p2 = comp_ptr.0.wrapping_offset(low as isize);
                            }
                        }
                    }
                }

                for v in graph.in_neigh(u).skip(neighbor_rounds) {
                    for v in graph.out_neigh(u).skip(neighbor_rounds) {
                        let mut p1 = comp_ptr.0.wrapping_offset(u as isize);
                        let mut p2 = comp_ptr.0.wrapping_offset(v.as_node() as isize);

                        while unsafe { *p1 != *p2 } {
                            let (high, low) = unsafe {
                                if *p1 > *p2 {
                                    (*p1, *p2)
                                } else {
                                    (*p2, *p1)
                                }
                            };

                            let p_high = comp_ptr.0.wrapping_offset(high as isize);

                            if unsafe { *p_high == low } {
                                break;
                            }

                            if unsafe {
                                (*p_high == high)
                                    && (*comp_ptr.0.wrapping_offset(high as isize) == high)
                            } {
                                // FIXME: They use atomic CAS here, but it should not be needed
                                // for single-threaded execution.
                                unsafe {
                                    *comp_ptr.0.wrapping_offset(high as isize) = low;
                                }
                                break;
                            }

                            unsafe {
                                p1 = comp_ptr.0.wrapping_offset(
                                    (*comp_ptr.0.wrapping_offset(high as isize)) as isize,
                                );
                                p2 = comp_ptr.0.wrapping_offset(low as isize);
                            }
                        }
                    }
                }
            }
        });
    }

    // Finally, `compress` for final convergence
    compress_mt(graph, &mut comp);
    comp
}

fn verifier<'a, V: AsNode, E: AsNode, G: CSRGraph<V, E>>(graph: &G, comp: &Vec<NodeId>) -> bool {
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
