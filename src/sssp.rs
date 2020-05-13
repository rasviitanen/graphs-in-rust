//! Returns array of distances for all vertices from given source vertex
//! This SSSP implementation makes use of the *∆-stepping algorithm* \[1\]. The type
//! used for weights and distances (WeightT) is typedefined in benchmark.h. The
//! delta parameter `(-d)` should be set for each input graph.
//!
//! The bins of width delta are actually all thread-local and of type std::vector
//! so they can grow but are otherwise capacity-proportional. Each iteration is
//! done in two phases separated by barriers. In the first phase, the current
//! shared bin is processed by all threads. As they find vertices whose distance
//! they are able to improve, they add them to their thread-local bins. During this
//! phase, each thread also votes on what the next bin should be (smallest
//! non-empty bin). In the next phase, each thread copies their selected
//! thread-local bin into the shared bin.
//!
//! Once a vertex is added to a bin, it is not removed, even if its distance is
//! later updated and it now appears in a lower bin. We find ignoring vertices if
//! their current distance is less than the min distance for the bin to remove
//! enough redundant work that this is faster than removing the vertex from older
//! bins.
//!
//! ## Sources
//! \[1\] Ulrich Meyer and Peter Sanders. "δ-stepping: a parallelizable shortest path
//!     algorithm." Journal of Algorithms, 49(1):114–152, 2003.

use crate::benchmark::SourcePicker;
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
use std::collections::HashSet;

const K_DIST_INF: Weight = std::usize::MAX / 2;
const K_MAX_BIN: usize = std::usize::MAX / 2;

pub fn delta_step<'a, V: AsNode, E: WeightedEdge + AsNode, G: CSRGraph<V, E>>(
    graph: &G,
    source: NodeId,
    delta: Weight,
) -> Vec<Weight> {
    let mut dist = vec![K_DIST_INF; graph.num_nodes()];
    dist[source] = 0;
    let mut frontier = vec![0; graph.num_edges_directed()];

    let mut shared_indices: [usize; 2] = [0, K_MAX_BIN];
    let mut frontier_tails: [usize; 2] = [1, 0];

    frontier[0] = source;

    let mut local_bins: Vec<Vec<NodeId>> = Vec::new();
    let mut iter = 0;

    while shared_indices[iter & 1] != K_MAX_BIN {
        let curr_bin_index = unsafe { &mut shared_indices[iter & 1] as *mut _ };
        let next_bin_index = unsafe { &mut shared_indices[(iter + 1) & 1] as *mut _ };
        let curr_frontier_tail = unsafe { &mut frontier_tails[iter & 1] as *mut _ };
        let next_frontier_tail = unsafe { &mut frontier_tails[(iter + 1) & 1] as *mut _ };

        (0..unsafe { *curr_frontier_tail })
            .into_iter()
            .for_each(|i| {
                let u = frontier[i];
                if dist[u] >= delta * (unsafe { *curr_bin_index }) {
                    for wn in graph.out_neigh(u) {
                        let mut old_dist = dist[wn.as_node()];
                        let mut new_dist = dist[u] + wn.get_weight();

                        if new_dist < old_dist {
                            let mut changed_dist = true;

                            while {
                                //FIXME: replace with CAS
                                let mut cas_status = false;
                                if dist[wn.as_node()] == old_dist {
                                    dist[wn.as_node()] = new_dist;
                                    cas_status = true;
                                }
                                !cas_status
                            } {
                                old_dist = dist[wn.as_node()];
                                if old_dist <= new_dist {
                                    changed_dist = false;
                                    break;
                                }
                            }

                            if changed_dist {
                                let dest_bin = new_dist / delta;
                                if dest_bin >= local_bins.len() {
                                    local_bins.resize(dest_bin + 1, Vec::new());
                                }
                                local_bins[dest_bin].push(wn.as_node());
                            }
                        }
                    }
                }
            });

        for i in (unsafe { *curr_bin_index })..local_bins.len() {
            if !local_bins[i].is_empty() {
                unsafe {
                    *next_bin_index = usize::min(*next_bin_index, i);
                }
                break;
            }
        }

        unsafe {
            *curr_bin_index = K_MAX_BIN;
            *curr_frontier_tail = 0;
        }

        if unsafe { *next_bin_index } < local_bins.len() {
            let copy_start = unsafe { *next_frontier_tail };
            unsafe {
                *next_frontier_tail += local_bins[unsafe { *next_bin_index }].len();
            } // FIXME: fetch-and-add

            for e in frontier
                .iter_mut()
                .skip(copy_start)
                .zip(local_bins[unsafe { *next_bin_index }].iter())
            {
                *e.0 = *e.1
            }

            local_bins[unsafe { *next_bin_index }].resize(0, 0);
        }

        iter += 1;
    }

    dist
}

#[derive(Copy, Clone)]
struct UnsafeBox(*mut usize);

unsafe impl Send for UnsafeBox {}
unsafe impl Sync for UnsafeBox {}

pub fn delta_step_mt<'a, V: AsNode, E: WeightedEdge + AsNode, G: Send + Sync + CSRGraph<V, E>>(
    graph: &G,
    source: NodeId,
    delta: Weight,
) -> Vec<Weight> {
    let mut dist = vec![K_DIST_INF; graph.num_nodes()];
    let dist_ptr: UnsafeBox = UnsafeBox(dist.as_mut_ptr());

    dist[source] = 0;
    let mut frontier = vec![0; graph.num_edges_directed()];

    let mut shared_indices: [usize; 2] = [0, K_MAX_BIN];
    let mut frontier_tails: [usize; 2] = [1, 0];

    frontier[0] = source;

    let local_bins: std::sync::Arc<std::sync::Mutex<Vec<Vec<NodeId>>>> =
        std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));

    let mut iter = 0;

    while shared_indices[iter & 1] != K_MAX_BIN {
        let curr_bin_index = unsafe { UnsafeBox(&mut shared_indices[iter & 1] as *mut _) };
        let next_bin_index = unsafe { UnsafeBox(&mut shared_indices[(iter + 1) & 1] as *mut _) };
        let curr_frontier_tail = unsafe { UnsafeBox(&mut frontier_tails[iter & 1] as *mut _) };
        let next_frontier_tail =
            unsafe { UnsafeBox(&mut frontier_tails[(iter + 1) & 1] as *mut _) };

        (0..unsafe { *curr_frontier_tail.0 })
            .into_par_iter()
            .for_each(|i| {
                let u = frontier[i as usize];
                if dist[u] >= delta * (unsafe { *curr_bin_index.0 }) {
                    for wn in graph.out_neigh(u) {
                        let mut old_dist = dist[wn.as_node()];
                        let new_dist = dist[u] + wn.get_weight();

                        if new_dist < old_dist {
                            let mut changed_dist = true;

                            while {
                                //FIXME: replace with CAS
                                let mut cas_status = false;
                                if dist[wn.as_node()] == old_dist {
                                    unsafe {
                                        *dist_ptr.0.wrapping_offset(wn.as_node() as isize) =
                                            new_dist;
                                    }
                                    cas_status = true;
                                }
                                !cas_status
                            } {
                                old_dist = dist[wn.as_node()];
                                if old_dist <= new_dist {
                                    changed_dist = false;
                                    break;
                                }
                            }

                            if changed_dist {
                                let dest_bin = new_dist / delta;
                                let mut local_bins = local_bins.lock().unwrap();
                                if dest_bin >= local_bins.len() {
                                    local_bins.resize(dest_bin + 1, Vec::new());
                                }
                                local_bins[dest_bin].push(wn.as_node());
                            }
                        }
                    }
                }
            });

        let mut local_bins = local_bins.lock().unwrap();
        for i in (unsafe { *curr_bin_index.0 })..local_bins.len() {
            if !local_bins[i].is_empty() {
                unsafe {
                    *next_bin_index.0 = usize::min(*next_bin_index.0, i);
                }
                break;
            }
        }

        unsafe {
            *curr_bin_index.0 = K_MAX_BIN;
            *curr_frontier_tail.0 = 0;
        }

        if unsafe { *next_bin_index.0 } < local_bins.len() {
            let copy_start = unsafe { *next_frontier_tail.0 };
            unsafe {
                *next_frontier_tail.0 += local_bins[unsafe { *next_bin_index.0 }].len();
            } // FIXME: fetch-and-add

            for e in frontier
                .iter_mut()
                .skip(copy_start)
                .zip(local_bins[unsafe { *next_bin_index.0 }].iter())
            {
                *e.0 = *e.1
            }

            local_bins[unsafe { *next_bin_index.0 }].resize(0, 0);
        }

        iter += 1;
    }

    dist
}
