use crate::types::*;
use rand::prelude::*;
use crate::graph::CSRGraph;


struct SourcePicker<G: CSRGraph> {
    given_source: Option<NodeId>,
    rng: rand::rngs::ThreadRng,
    udist: rand::distributions::Uniform<usize>,
    graph: G,
}

impl<G: CSRGraph> SourcePicker<G> {
    pub fn new(graph: G, given_source: NodeId) -> Self {
        Self {
            given_source: None,
            rng: rand::thread_rng(),
            udist: rand::distributions::Uniform::from(0..graph.num_nodes()),
            graph,
        }
    }

    pub fn from_source(graph: G, given_source: NodeId) -> Self {
        Self {
            given_source: Some(given_source),
            rng: rand::thread_rng(),
            udist: rand::distributions::Uniform::from(0..graph.num_nodes()),
            graph,
        }
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
}

type GraphFunc<T> = Box<dyn Fn(&T) -> ()>;
type AnalysisFunc = Box<dyn Fn() -> ()>;
type VerifyFunc = Box<dyn Fn() -> ()>;

const NUM_TRIALS: usize = 10;

pub fn benchmark_kernel<G: CSRGraph>(
    graph: G,
    kernel: GraphFunc<G>,
    stats: AnalysisFunc,
    verify: VerifyFunc,
) {
    graph.print_stats();

    let mut total_time = 0;

    for iter in 0..NUM_TRIALS {
        let tStart = time::now_utc();
        let result = kernel(&graph);
        let tFinish = time::now_utc();
        total_time = (tStart - tFinish).num_milliseconds();
        println!("\tTrial time {} msec", total_time);
    }

    println!("\tBenchmark took {} msec", total_time);
}