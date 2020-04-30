use gapbs::benchmark::SourcePicker;
use gapbs::bfs;
use gapbs::builder::BuilderBase;
use gapbs::graphmodels;
use gapbs::types::*;

fn main() {
    const START_VERTEX: NodeId = 3;
    let mut builder = BuilderBase::new();
    let graph: graphmodels::rc::Graph<usize> = builder.make_graph();
    let mut source_picker = SourcePicker::new(graph, START_VERTEX);

    // source_picker.benchmark_kernel_bfs(
    //     Box::new(|| {}),
    //     Box::new(|| {}),
    // );

    source_picker.benchmark_kernel_pr();
}
