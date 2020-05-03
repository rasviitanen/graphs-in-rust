use gapbs::benchmark::SourcePicker;
use gapbs::bfs;
use gapbs::builder::BuilderBase;
use gapbs::graphmodels;
use gapbs::types::*;

fn main() {
    let mut builder = BuilderBase::new();

    // Choose graph model here
    let graph: graphmodels::rc::Graph<usize> = builder.make_graph();
    let mut source_picker = SourcePicker::new(graph);

    // source_picker.benchmark_kernel_bfs(
    //     Box::new(|| {}),
    //     Box::new(|| {}),
    // );
    // source_picker.benchmark_kernel_tc();
    // source_picker.benchmark_kernel_cc();
    source_picker.benchmark_kernel_tc();

}
