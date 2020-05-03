use crate::generator::Generator;
use crate::graph::CSRGraph;
use crate::types::*;
use rayon::iter::IndexedParallelIterator;
use rayon::iter::IntoParallelIterator;
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::IntoParallelRefMutIterator;
use rayon::iter::ParallelExtend;
use rayon::iter::ParallelIterator;
use std::sync::atomic::{AtomicUsize, Ordering};

const SYMMETRIZE: bool = false;
const UNIFORM: bool = true;
const NEEDS_WEIGHTS: bool = false;
const FILE_NAME: &'static str = "datasets/dolphins.out"; // "datasets/dolphins.out"
const INVERT: bool = false;
const SCALE: usize = 3;
const DEGREE: usize = 1;

pub struct BuilderBase {
    symmetrize: bool,
    needs_weights: bool,
    num_nodes: Option<usize>,
}

impl BuilderBase {
    pub fn new() -> Self {
        Self {
            symmetrize: SYMMETRIZE,
            num_nodes: None,
            needs_weights: NEEDS_WEIGHTS, // FIXME:  This is wrong
        }
    }

    pub fn get_source(e: Edge) -> DestId {
        e.0 // FIXME: e.1 ?
    }

    pub fn find_max_node_id(edge_list: &EdgeList) -> usize {
        let mut max_seen = 0;
        edge_list.iter().for_each(|e| {
            max_seen = std::cmp::max(max_seen, e.0);
            max_seen = std::cmp::max(max_seen, e.1);
        });

        max_seen
    }

    pub fn count_degrees(&self, edge_list: &EdgeList, transpose: bool) -> Vec<usize> {
        let mut degrees: Vec<AtomicUsize> = (0..self.num_nodes.expect("`num_nodes` is not set"))
            .into_iter()
            .map(|_| AtomicUsize::new(0))
            .collect();

        edge_list.par_iter().for_each(|e| {
            if self.symmetrize || (!self.symmetrize && !transpose) {
                degrees[e.0].fetch_add(1, Ordering::SeqCst);
            }

            if self.symmetrize || (!self.symmetrize && transpose) {
                degrees[e.1].fetch_add(1, Ordering::SeqCst);
            }
        });

        degrees.drain(..).map(|d| d.into_inner()).collect()
    }

    pub fn make_graph_from_edge_list<V: AsNode, E: AsNode, G: CSRGraph<V, E>>(
        &mut self,
        edge_list: &EdgeList,
    ) -> G {
        let t_start = time::now_utc();

        if self.num_nodes.is_none() {
            self.num_nodes = Some(Self::find_max_node_id(edge_list) + 1);
        }

        if self.needs_weights {
            unimplemented!("Weights are not yet supported");
        }

        let graph;
        if self.symmetrize {
            println!("Building directed");
            graph = G::build_directed(
                self.num_nodes.expect("`num_nodes` is not specified"),
                edge_list,
            )
        } else {
            println!("Building undirected");
            graph = G::build_undirected(
                self.num_nodes.expect("`num_nodes` is not specified"),
                edge_list,
            )
        }

        println!("GRAPH BUILDING IS DONE");
        let t_finish = time::now_utc();
        println!(
            "\tBuild Time: {} msec",
            (t_finish - t_start).num_milliseconds()
        );

        graph
    }

    fn squish_csr<V: AsNode, E: AsNode, G: CSRGraph<V, E>>(graph: &mut G, transpose: bool) {
        for v in graph.vertices() {
            let mut neighs: Vec<_>;
            if transpose {
                neighs = graph.in_neigh(v.as_node()).collect();
                neighs.sort_by(|a, b| a.as_node().partial_cmp(&b.as_node()).unwrap());
                neighs.dedup_by(|a, b| a.as_node() == b.as_node());
                neighs.retain(|e| e.as_node() != v.as_node());
                graph.replace_in_edges(v.as_node(), neighs);
            } else {
                neighs = graph.out_neigh(v.as_node()).collect();
                neighs.sort_by(|a, b| a.as_node().partial_cmp(&b.as_node()).unwrap());
                neighs.dedup_by(|a, b| a.as_node() == b.as_node());
                neighs.retain(|e| e.as_node() != v.as_node());
                graph.replace_out_edges(v.as_node(), neighs);
            }
        }
    }

    fn squish_graph<V: AsNode, E: AsNode, G: CSRGraph<V, E>>(&self, graph: &mut G) {
        Self::squish_csr(graph, false);
        if graph.directed() {
            if INVERT {
                Self::squish_csr(graph, true);
            }
        }
    }

    pub fn make_graph<V: AsNode, E: AsNode, G: CSRGraph<V, E>>(&mut self) -> G {
        let edge_list;
        let generator = Generator::new(SCALE, DEGREE);
        if FILE_NAME != "" {
            dbg!("Generating edge list from file");
            edge_list = generator.generate_edge_list_from_file(FILE_NAME);
        } else {
            edge_list = generator.generate_edge_list(UNIFORM);
        }

        let mut graph = self.make_graph_from_edge_list(&edge_list);
        self.squish_graph(&mut graph);
        graph
    }

    // pub fn make_csr<G: CSRGraph>(
    //     &self,
    //     edge_list: &EdgeList,
    //     transpose: bool,
    // ) -> G {
    //     // let degrees = self.count_degrees(edge_list, transpose);
    //     let num_nodes = self.num_nodes.expect("Num nodes not set");
    //     let mut neighs = Vec::with_capacity(num_nodes);
    //     neighs.par_extend(
    //         edge_list
    //             .par_iter()
    //             .map(|e| {
    //                 if self.symmetrize || (!self.symmetrize && !transpose) {
    //                     return e.1;
    //                 }

    //                 if self.symmetrize || (!self.symmetrize && transpose) {
    //                     unimplemented!("Should call GetSource(e)");
    //                 }

    //                 unreachable!("OOPS, should not be reachable");
    //             })
    //     )
    // }
}
