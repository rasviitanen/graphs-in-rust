use gapbs::benchmark::{benchmark_kernel, benchmark_kernel_with_sp, SourcePicker};
use gapbs::bfs;
use gapbs::builder::BuilderBase;
use gapbs::graphmodels;
use gapbs::types::*;

type Graph = graphmodels::rc::Graph<usize>;

fn main() {
    println!(
        r#"
--------------------------------------------------------------------
--------------------------------------------------------------------
                            EXECUTING
                    The GAP Benchmarking Suite
                            (Rust port)
--------------------------------------------------------------------
--------------------------------------------------------------------
        "#
    );

    let mut builder = BuilderBase::new();
    let graph: Graph = builder.make_graph();
    let mut source_picker = SourcePicker::new(&graph);
    let mut source_picker1 = SourcePicker::new(&graph);
    let mut source_picker2 = SourcePicker::new(&graph);
    let mut source_picker3 = SourcePicker::new(&graph);

    println!("Breadth-First Search (BFS) - direction optimizing");
    benchmark_kernel_with_sp(
        &graph,
        &mut source_picker1,
        Box::new(|g: &Graph, sp| {
            gapbs::bfs::do_bfs(g, sp.pick_next());
        }),
        Box::new(|| {}),
        Box::new(|| {}),
    );

    println!("Single-Source Shortest Paths (SSSP) - delta stepping");
    benchmark_kernel_with_sp(
        &graph,
        &mut source_picker3,
        Box::new(|g: &Graph, sp| {
            gapbs::sssp::delta_step(g, sp.pick_next(), 1);
        }),
        Box::new(|| {}),
        Box::new(|| {}),
    );

    println!("Triangle Counting (TC) - Order invariant with possible relabelling");
    source_picker.benchmark_kernel_tc();

    println!("Connected Components (CC) - Afforest & Shiloach-Vishkin");
    source_picker.benchmark_kernel_cc();

    println!("PageRank (PR) - iterative method in pull direction");
    source_picker.benchmark_kernel_pr();

    println!("Betweenness Centrality (BC) - Brandes");
    benchmark_kernel_with_sp(
        &graph,
        &mut source_picker2,
        Box::new(|g: &Graph, mut sp| {
            gapbs::bc::brandes(g, &mut sp, 1);
        }),
        Box::new(|| {}),
        Box::new(|| {}),
    );


}
