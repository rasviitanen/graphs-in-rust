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
use std::sync::atomic::{AtomicUsize, Ordering};
use rayon::iter::IndexedParallelIterator;
use rayon::iter::IntoParallelIterator;
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::IntoParallelRefMutIterator;
use rayon::iter::ParallelExtend;
use rayon::iter::ParallelIterator;
use rayon::slice::ParallelSlice;
use rand::prelude::*;

pub fn ops<'a, V: AsNode, E: AsNode, G: CSRGraph<V, E>>(graph: &G) {
    let rnd_id = || thread_rng().gen_range(1, graph.num_nodes()-1);

    (0..1000).into_iter().for_each(|_| {
        let mut rng = thread_rng();
        match rng.gen_range(1, 100) {
            1..=25 => {
                graph.op_add_vertex(rnd_id());
            }
            // 26..=75 => {
            //     // graph.op_add_edge(rnd_id(), rnd_id());
            // }
            // 51..=75 => {
            //     graph.op_delete_edge(black_box(rnd_id()), black_box(rnd_id()));
            // }
            // 76..=100 => {
            //     graph.op_delete_vertex(black_box(rnd_id()));
            // }
            _ => {
                graph.op_add_vertex(rnd_id());
                // graph.op_find_vertex(rnd_id());
            }
        }
    });
}

pub fn ops_mt<'a, V: AsNode, E: AsNode, G: Send + Sync + CSRGraph<V, E>>(graph: &G) {
    let rnd_id = || thread_rng().gen_range(1, graph.num_nodes()-1);

    (0..1000).into_par_iter().for_each(|_| {
        let mut rng = thread_rng();
        match rng.gen_range(1, 100) {
            1..=25 => {
                // dbg!("add vx");
                graph.op_add_vertex(rnd_id());
            }
            // 50..=75 => {
            //     // dbg!("add ed");
            //     // graph.op_add_edge(rnd_id(), rnd_id());
            // }
            // 51..=75 => {
            //     graph.op_delete_edge(black_box(rnd_id()), black_box(rnd_id()));
            // }
            // 76..=100 => {
            //     graph.op_delete_vertex(black_box(rnd_id()));
            // }
            _ => {
                // dbg!("fnd");
                graph.op_add_vertex(rnd_id());
                // graph.op_find_vertex(rnd_id());
            }
        }
    });
}