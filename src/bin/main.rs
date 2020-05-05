use gapbs::benchmark::{benchmark_kernel, benchmark_kernel_with_sp, SourcePicker};
use gapbs::bfs;
use gapbs::builder::BuilderBase;
use gapbs::graphmodels;
use gapbs::types::*;

type Graph = graphmodels::rc::Graph<usize>;

fn main() {
    let mut builder = BuilderBase::new();

    let graph: Graph = builder.make_graph();
    let mut source_picker = SourcePicker::new(&graph);

    // source_picker.benchmark_kernel_bfs(
    //     Box::new(|| {}),
    //     Box::new(|| {}),
    // );
    // source_picker.benchmark_kernel_tc();
    // source_picker.benchmark_kernel_cc();
    // source_picker.benchmark_kernel_tc();

    // BC
    // benchmark_kernel_with_sp(
    //     &graph,
    //     &mut source_picker,
    //     Box::new(|g: &Graph, mut sp| {
    //         gapbs::bc::brandes(g, &mut sp, 1);
    //     }),
    //     Box::new(|| {}),
    //     Box::new(|| {}),
    // );

    // SSSP
    benchmark_kernel_with_sp(
        &graph,
        &mut source_picker,
        Box::new(|g: &Graph, mut sp| {
            gapbs::sssp::delta_step(g, 5, 1);
        }),
        Box::new(|| {}),
        Box::new(|| {}),
    );
}
