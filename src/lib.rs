#![feature(cell_update)]
#![feature(dropck_eyepatch)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_macros)]
#![allow(unused_variables)]
#![allow(unused_unsafe)]

pub const K_RAND_SEED: usize = 52;

#[macro_use]
extern crate gc_derive;
extern crate gc;

extern crate crossbeam_epoch as epoch;
extern crate crossbeam_utils as utils;

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

pub mod ops;

mod timer;

pub mod treenodes;
