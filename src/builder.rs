use crate::types::*;
use crate::graph::CSRGraph;
use crate::generator::Generator;
use std::sync::atomic::{Ordering, AtomicUsize};
use rayon::iter::IndexedParallelIterator;
use rayon::iter::IntoParallelIterator;
use rayon::iter::IntoParallelRefMutIterator;
use rayon::iter::ParallelExtend;
use rayon::iter::ParallelIterator;
use rayon::iter::IntoParallelRefIterator;

const SYMMETRIZE: bool = false;
const UNIFORM: bool = true;
const NEEDS_WEIGHTS: bool = false;
const FILE_NAME: &'static str = "";
const SCALE: usize = 2;
const DEGREE: usize = 2;


struct BuilderBase {
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
        let mut max_seen = AtomicUsize::new(0);
        edge_list.par_iter().for_each(|e| {
            let current_max = max_seen.load(Ordering::SeqCst);
            max_seen.store(std::cmp::max(current_max, e.0), Ordering::SeqCst);
            max_seen.store(std::cmp::max(current_max, e.1), Ordering::SeqCst);
        });

        max_seen.into_inner()
    }

    pub fn count_degrees(&self, edge_list: &EdgeList, transpose: bool) -> Vec<usize> {
        let mut degrees: Vec<AtomicUsize> = (0..self.num_nodes.expect("`num_nodes` is not set")).into_iter().map(|_| AtomicUsize::new(0)).collect();

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


    pub fn make_graph_from_edge_list(&self, edge_list: &EdgeList) {
        if self.num_nodes.is_none() {
            self.num_nodes = Some(Self::find_max_node_id(edge_list));
        }

        if self.needs_weights {
            unimplemented!("Weights are not yet supported");
        }

        unimplemented!("continue here");
    }

    pub fn make_graph<G: CSRGraph>() -> G {
        let edge_list;
        if FILE_NAME != "" {
            unimplemented!("Loading from file is not supported");
        } else {
            let generator = Generator::new(SCALE, DEGREE);
            edge_list = generator.generate_edge_list(UNIFORM);
        }

        unimplemented!("continue here");
    }

    pub fn make_csr(
        &self,
        edge_list: &EdgeList,
        transpose: bool,
        index: Box<DestId>,
        neighs: Box<DestId>,
    ) {
        // let degrees = self.count_degrees(edge_list, transpose);
        let num_nodes = self.num_nodes.expect("Num nodes not set");
        let mut neighs = Vec::with_capacity(num_nodes);
        neighs.par_extend(
            edge_list
                .par_iter()
                .map(|e| {
                    if self.symmetrize || (!self.symmetrize && !transpose) {
                        return e.1;
                    }
        
                    if self.symmetrize || (!self.symmetrize && transpose) {
                        unimplemented!("Should call GetSource(e)");
                    }

                    unreachable!("OOPS, should not be reachable");
                })
        )
    }
}