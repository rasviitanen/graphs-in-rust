use crate::graph::CSRGraph;
use crate::types::*;
use rand::prelude::*;
use std::marker::PhantomData;

type GraphFunc<T> = Box<dyn FnMut(&T) -> ()>;
type GraphFuncTwo<T, E> = Box<dyn FnMut(&T, &mut E) -> ()>;
type AnalysisFunc = Box<dyn Fn() -> ()>;
type VerifyFunc = Box<dyn Fn() -> ()>;

pub struct SourcePicker<'a, V: AsNode, E: AsNode, G: CSRGraph<'a, V, E>> {
    given_source: Option<NodeId>,
    rng: rand::rngs::ThreadRng,
    udist: rand::distributions::Uniform<usize>,
    graph: &'a G,
    _phantom: PhantomData<(V, E)>,
}

impl<'a, V: AsNode, E: AsNode, G: CSRGraph<'a, V, E>> SourcePicker<'a, V, E, G> {
    pub fn new(graph: &'a G) -> Self {
        Self {
            given_source: None,
            rng: rand::thread_rng(),
            udist: rand::distributions::Uniform::from(0..graph.num_nodes()),
            graph,
            _phantom: PhantomData,
        }
    }

    pub fn from_source(graph: &'a G, given_source: NodeId) -> Self {
        Self {
            given_source: Some(given_source),
            rng: rand::thread_rng(),
            udist: rand::distributions::Uniform::from(0..graph.num_nodes()),
            graph,
            _phantom: PhantomData,
        }
    }

    /// Picks a vertex from the graph using a uniform distribution
    ///
    /// Loops infinitely if the picked vertex does not exist, or if it
    /// does not have any edges.
    pub fn pick_next(&mut self) -> NodeId {
        if let Some(gs) = self.given_source {
            return gs;
        }

        loop {
            let source = self.udist.sample(&mut self.rng);
            if self.graph.out_degree(source) != 0 {
                return source;
            }
        }
    }

    /// Benchmarks PageRank with `max_iters = 20` and `epsilon = 0.0004`
    pub fn benchmark_kernel_pr(&self) {
        benchmark_kernel(
            self.graph,
            Box::new(|g: &G| {
                crate::pr::page_rank_pull(g, 20, Some(0.0004));
            }),
            Box::new(|| {}),
            Box::new(|| {}),
        );
    }

    /// Triangle Counting (TC) - Order invariant with possible relabelling
    /// FIXME: Relabelling is not supported yet
    pub fn benchmark_kernel_tc(&self) {
        benchmark_kernel(
            self.graph,
            Box::new(|g: &G| crate::tc::hybrid(g)),
            Box::new(|| {}),
            Box::new(|| {}),
        );
    }

    pub fn benchmark_kernel_cc(&self) {
        benchmark_kernel(
            self.graph,
            Box::new(|g: &G| {
                crate::cc::afforest(g, None);
            }),
            Box::new(|| {}),
            Box::new(|| {}),
        );
    }
}

/// Benchmarks a given `kernel`
pub fn benchmark_kernel<'a, V: AsNode, E: AsNode, G: CSRGraph<'a, V, E>>(
    graph: &G,
    mut kernel: GraphFunc<G>,
    stats: AnalysisFunc,
    verify: VerifyFunc,
) {
    // graph.print_stats();
    let mut timer = crate::timer::ScopedTimer::new("BENCHMARK");

    for i in 1..=NUM_TRIALS {
        timer.checkpoint(&format!("Trial {}", i));
        let result = kernel(&graph);
        timer.elapsed_since_checkpoint();
    }
}

/// Benchmarks a given `kernel`
pub fn benchmark_kernel_with_sp<'a, V: AsNode, E: AsNode, G: CSRGraph<'a, V, E>>(
    graph: &'a G,
    source_picker: &'a mut SourcePicker<'a, V, E, G>,
    mut kernel: GraphFuncTwo<G, SourcePicker<'a, V, E, G>>,
    stats: AnalysisFunc,
    verify: VerifyFunc,
) {
    // graph.print_stats();
    let mut timer = crate::timer::ScopedTimer::new("BENCHMARK");

    for i in 1..=NUM_TRIALS {
        timer.checkpoint(&format!("Trial {}", i));
        let result = kernel(&graph, source_picker);
        timer.elapsed_since_checkpoint();
    }
}
