#![feature(cell_update)]
pub const K_RAND_SEED: usize = 27491095;

pub mod benchmark;
pub mod generator;
pub mod types;
pub mod graph;
pub mod graphmodels;
pub mod builder;
pub mod slidingqueue;

// Breadth-First Search (BFS) - direction optimizing
pub mod bfs;
// Single-Source Shortest Paths (SSSP) - delta stepping
pub mod sssp;
// PageRank (PR) - iterative method in pull direction
pub mod pr;
// Connected Components (CC) - Afforest & Shiloach-Vishkin
pub mod cc;
// Betweenness Centrality (BC) - Brandes
pub mod bc;
// Triangle Counting (TC) - Order invariant with possible relabelling
pub mod tc;