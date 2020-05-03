#![feature(cell_update)]
pub const K_RAND_SEED: usize = 27491095;

/// Performs the benchmarks
pub mod benchmark;
/// Builds and squishes a graph (removes self-references and parallel edges)
pub mod builder;
/// Generates or loads edge lists `Vec<(v, e)>` of different distributions
pub mod generator;
/// Graph trait that is used in all implementations, any memory model is required to implement it
pub mod graph;
/// Different graph models, `Rc`, `Gc`, `Cc`, `Epoch`, `Arena`...
pub mod graphmodels;
/// A sliding queue implementation using iterators
pub mod slidingqueue;
/// Common type sfor edges, vertices and collections of them.
pub mod types;

/// # Betweenness Centrality (BC) - Brandes
pub mod bc;
/// # Breadth-First Search (BFS) - direction optimizing
pub mod bfs;
/// # Connected Components (CC) - Afforest & Shiloach-Vishkin
pub mod cc;
/// # PageRank (PR) - iterative method in pull direction
pub mod pr;
/// # Single-Source Shortest Paths (SSSP) - delta stepping
pub mod sssp;
/// # Triangle Counting (TC) - Order invariant with possible relabelling
pub mod tc;
