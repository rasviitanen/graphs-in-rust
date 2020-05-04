use crate::graph::CSRGraph;
use crate::types::*;
use rand::prelude::*;
use std::marker::PhantomData;

const NUM_TRIALS: usize = 1;

type GraphFunc<T> = Box<dyn FnMut(&T) -> ()>;
type GraphFuncTwo<T, E> = Box<dyn FnMut(&T, &mut E) -> ()>;
type AnalysisFunc = Box<dyn Fn() -> ()>;
type VerifyFunc = Box<dyn Fn() -> ()>;

pub struct SourcePicker<'a, V: AsNode, E: AsNode, G: CSRGraph<V, E>> {
    given_source: Option<NodeId>,
    rng: rand::rngs::ThreadRng,
    udist: rand::distributions::Uniform<usize>,
    graph: &'a G,
    _phantom: PhantomData<(V, E)>,
}

impl<'a, V: AsNode, E: AsNode, G: CSRGraph<V, E>> SourcePicker<'a, V, E, G> {
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

    /// Executes BFS with a suitable start node
    pub fn bfs_bound(&mut self) {
        let next = self.pick_next();
        self.graph.old_bfs(next);
        println!("-.-.-.-.-.-.-.-.-.-");
        crate::bfs::do_bfs(self.graph, next);
    }

    /// Benchmarks BFS (direction optimizing)
    pub fn benchmark_kernel_bfs(&mut self, stats: AnalysisFunc, verify: VerifyFunc) {
        self.graph.print_stats();
        let mut total_time = 0;

        for iter in 0..NUM_TRIALS {
            let t_start = time::now_utc();
            let result = self.bfs_bound();
            let t_finish = time::now_utc();
            total_time = (t_finish - t_start).num_milliseconds();
            println!("\tTrial time {} msec", total_time);
        }

        println!("\tBenchmark took {} msec", total_time);
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
pub fn benchmark_kernel<V: AsNode, E: AsNode, G: CSRGraph<V, E>>(
    graph: &G,
    mut kernel: GraphFunc<G>,
    stats: AnalysisFunc,
    verify: VerifyFunc,
) {
    graph.print_stats();
    let mut total_time = 0;

    for iter in 0..NUM_TRIALS {
        let t_start = time::now_utc();
        let result = kernel(&graph);
        let t_finish = time::now_utc();
        total_time += (t_finish - t_start).num_milliseconds();
        println!("\tTrial time {} msec", total_time);
    }

    println!("\tBenchmark took {} msec", total_time);
}

/// Benchmarks a given `kernel`
pub fn benchmark_kernel_with_sp<'a, V: AsNode, E: AsNode, G: CSRGraph<V, E>>(
    graph: &'a G,
    source_picker: &'a mut SourcePicker<'a, V, E, G>,
    mut kernel: GraphFuncTwo<G, SourcePicker<'a, V, E, G>>,
    stats: AnalysisFunc,
    verify: VerifyFunc,
) {
    graph.print_stats();
    let mut total_time = 0;

    for iter in 0..NUM_TRIALS {
        let t_start = time::now_utc();
        let result = kernel(&graph, source_picker);
        let t_finish = time::now_utc();
        total_time += (t_finish - t_start).num_milliseconds();
        println!("\tTrial time {} msec", total_time);
    }

    println!("\tBenchmark took {} msec", total_time);
}
