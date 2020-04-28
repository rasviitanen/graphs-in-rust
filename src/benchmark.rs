use crate::types::*;
use rand::prelude::*;
use crate::graph::CSRGraph;
use std::marker::PhantomData;

const NUM_TRIALS: usize = 1;

type GraphFunc<T> = Box<dyn FnMut(&T) -> ()>;
type AnalysisFunc = Box<dyn Fn() -> ()>;
type VerifyFunc = Box<dyn Fn() -> ()>;


pub struct SourcePicker<V: AsNode, E: AsNode, G: CSRGraph<V, E>> {
    given_source: Option<NodeId>,
    rng: rand::rngs::ThreadRng,
    udist: rand::distributions::Uniform<usize>,
    graph: G,
    _phantom: PhantomData<(V, E)>,
}

impl<V: AsNode, E: AsNode, G: CSRGraph<V, E>> SourcePicker<V, E, G> {
    pub fn new(graph: G, given_source: NodeId) -> Self {
        Self {
            given_source: None,
            rng: rand::thread_rng(),
            udist: rand::distributions::Uniform::from(0..graph.num_nodes()),
            graph,
            _phantom: PhantomData,
        }
    }

    pub fn from_source(graph: G, given_source: NodeId) -> Self {
        Self {
            given_source: Some(given_source),
            rng: rand::thread_rng(),
            udist: rand::distributions::Uniform::from(0..graph.num_nodes()),
            graph,
            _phantom: PhantomData,
        }
    }

    pub fn bfs_bound(&mut self) {
        let next = self.pick_next();
        self.graph.old_bfs(next);
        println!("#######################################");
        crate::bfs::do_bfs(&self.graph, next);
    }

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

    pub fn benchmark_kernel_bfs(
        &mut self,
        stats: AnalysisFunc,
        verify: VerifyFunc,
    ) {
        self.graph.print_stats();
        let mut total_time = 0;

        for iter in 0..NUM_TRIALS {
            let tStart = time::now_utc();
            let result = self.bfs_bound();
            let tFinish = time::now_utc();
            total_time = (tFinish- tStart).num_milliseconds();
            println!("\tTrial time {} msec", total_time);
        }

        println!("\tBenchmark took {} msec", total_time);
    }

    pub fn benchmark_kernel(
        &self,
        mut kernel: GraphFunc<G>,
        stats: AnalysisFunc,
        verify: VerifyFunc,
    ) {
        self.graph.print_stats();
        let mut total_time = 0;

        for iter in 0..NUM_TRIALS {
            let t_start = time::now_utc();
            let result = kernel(&self.graph);
            let t_finish = time::now_utc();
            total_time += (t_finish - t_start).num_milliseconds();
            println!("\tTrial time {} msec", total_time);
        }

        println!("\tBenchmark took {} msec", total_time);
    }
}
