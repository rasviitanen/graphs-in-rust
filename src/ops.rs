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

use crate::benchmark::SourcePicker;
use crate::graph::CSRGraph;
use crate::types::*;
use crossbeam_utils::thread;
use rand::prelude::*;
use rayon::iter::IndexedParallelIterator;
use rayon::iter::IntoParallelIterator;
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::IntoParallelRefMutIterator;
use rayon::iter::ParallelExtend;
use rayon::iter::ParallelIterator;
use rayon::slice::ParallelSlice;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use crate::timer::ScopedTimer;

pub fn ops<'a, V: AsNode, E: AsNode, G: CSRGraph<V, E>>(graph: &G) {
    let rnd_id = || thread_rng().gen_range(1, graph.num_nodes());

    (0..1_000).into_iter().for_each(|_| {
        let mut rng = thread_rng();

        match rng.gen_range(1, 100) {
            1..=40 => {
                graph.op_add_vertex(rnd_id());
            }
            41..=80 => {
                graph.op_delete_vertex(rnd_id());
            }
            81..=90 => {
                graph.op_add_edge(rnd_id(), rnd_id());
            }
            91..=100 => {
                graph.op_delete_edge(rnd_id(), rnd_id());
            }
            _ => {
                graph.op_find_vertex(rnd_id());
            }
        }
    });
}

pub fn ops_mt<'a, V: AsNode, E: AsNode, G: Send + Sync + CSRGraph<V, E>>(graph: &G) {
    let rnd_id = || thread_rng().gen_range(1, graph.num_nodes());
    // let graph = Arc::new(RwLock::new(graph));

    (0..1_000).into_par_iter().for_each(|_| {
        let mut rng = thread_rng();

        match rng.gen_range(1, 100) {
            1..=40 => {
                graph.op_add_vertex(rnd_id());
            }
            41..=80 => {
                graph.op_delete_vertex(rnd_id());
            }
            81..=90 => {
                graph.op_add_edge(rnd_id(), rnd_id());
            }
            91..=100 => {
                graph.op_delete_edge(rnd_id(), rnd_id());
            }
            _ => {
                graph.op_find_vertex(rnd_id());
            }
        }
    });
}

pub fn ops_epoch(graph: &crate::graphmodels::epoch::Graph<usize>) {
    let rnd_id = || thread_rng().gen_range(1, graph.num_nodes());
    (0..10).into_iter().for_each(|_| {
        let mut rng = thread_rng();
        let mut ops = Vec::new();
        for _ in 0..100 {
            match rng.gen_range(1, 100) {
                1..=40 => {
                    ops.push(crate::graphmodels::epoch::OpType::Insert(rnd_id(), None));
                }
                41..=80 => {
                    ops.push(crate::graphmodels::epoch::OpType::Delete(rnd_id()));
                }
                81..=90 => {
                    let edge_info = crate::graphmodels::epoch::EdgeInfo {
                        node_id: rnd_id(),
                        weight: None,
                    };

                    ops.push(crate::graphmodels::epoch::OpType::InsertEdge(rnd_id(), rnd_id(), Some(edge_info), false));
                }
                91..=100 => {
                    ops.push(crate::graphmodels::epoch::OpType::DeleteEdge(rnd_id(), rnd_id(), false));
                }
                _ => {
                    ops.push(crate::graphmodels::epoch::OpType::Find(rnd_id()));
                }
            }
        }
        graph.execute_ops(ops);
    });
}


pub fn ops_epoch_mt(graph: &crate::graphmodels::epoch::Graph<usize>) {
    let rnd_id = || thread_rng().gen_range(1, 1_000);

    (0..100).into_par_iter().for_each(|_| {
        let mut rng = thread_rng();
        let mut ops = Vec::new();
        for _ in 0..10 {
            match rng.gen_range(1, 100) {
                1..=40 => {
                    ops.push(crate::graphmodels::epoch::OpType::Insert(rnd_id(), None));
                }
                41..=80 => {
                    ops.push(crate::graphmodels::epoch::OpType::Delete(rnd_id()));
                }
                81..=90 => {
                    let edge_info = crate::graphmodels::epoch::EdgeInfo {
                        node_id: rnd_id(),
                        weight: None,
                    };

                    ops.push(crate::graphmodels::epoch::OpType::InsertEdge(rnd_id(), rnd_id(), Some(edge_info), false));
                }
                91..=100 => {
                    ops.push(crate::graphmodels::epoch::OpType::DeleteEdge(rnd_id(), rnd_id(), false));
                }
                _ => {
                    ops.push(crate::graphmodels::epoch::OpType::Find(rnd_id()));
                }
            }
        }
        graph.execute_ops(ops);
    });
}
