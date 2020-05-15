#![feature(custom_test_frameworks)]
#![test_runner(criterion::runner)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_macros)]
#![allow(unused_variables)]
#![allow(unused_unsafe)]

use criterion::{black_box, Criterion};
use criterion_macro::criterion;

use gapbs::benchmark::{benchmark_kernel, benchmark_kernel_with_sp, SourcePicker};
use gapbs::bfs;
use gapbs::builder::BuilderBase;
use gapbs::graph::CSRGraph;
use gapbs::graphmodels;
use gapbs::types::*;

type Graph<'a> = graphmodels::epoch::Graph<'a, usize>;

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

    // benchmark_kernel(
    //     &graph,
    //     Box::new(|g| {
    //         gapbs::ops::ops_mt(g);
    //     }),
    //     Box::new(|| {}),
    //     Box::new(|| {}),
    // );

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

    // println!("Single-Source Shortest Paths (SSSP) - delta stepping");
    // benchmark_kernel_with_sp(
    //     &graph,
    //     &mut source_picker2,
    //     Box::new(|g: &Graph, sp| {
    //         gapbs::sssp::delta_step_mt(g, sp.pick_next(), 1);
    //     }),
    //     Box::new(|| {}),
    //     Box::new(|| {}),
    // );

    // println!("Triangle Counting (TC) - Order invariant with possible relabelling");
    // source_picker.benchmark_kernel_tc();

    // println!("Connected Components (CC) - Afforest & Shiloach-Vishkin");
    // source_picker.benchmark_kernel_cc();

    // println!("Connected Components (CC) - Afforest & Shiloach-Vishkin");
    // benchmark_kernel(
    //     &graph,
    //     Box::new(|g: &Graph| {
    //         gapbs::cc::afforest_mt(g, None);
    //     }),
    //     Box::new(|| {}),
    //     Box::new(|| {}),
    // );

    // println!("PageRank (PR) - iterative method in pull direction");
    // source_picker.benchmark_kernel_pr();

    // println!("Betweenness Centrality (BC) - Brandes");
    // benchmark_kernel_with_sp(
    //     &graph,
    //     &mut source_picker3,
    //     Box::new(|g: &Graph, mut sp| {
    //         gapbs::bc::brandes(g, &mut sp, 1);
    //     }),
    //     Box::new(|| {}),
    //     Box::new(|| {}),
    // );
}

macro_rules! bench_generate {
    ($name: tt, $graphmodel: path, $group: expr) => {{
        use $graphmodel as graphmodel;
        $group.bench_function($name, |b| {
            let mut builder = BuilderBase::new();

            b.iter(|| {
                let graph: graphmodel::Graph<usize> = builder.make_graph();
            })
        });
    }};
}

macro_rules! bench_ops {
    ($name: tt, $graphmodel: path, $group: expr) => {{
        use $graphmodel as graphmodel;
        $group.bench_function($name, |b| {
            let mut builder = BuilderBase::new();
            let graph: graphmodel::Graph<usize> = builder.make_graph();
            b.iter(|| {
                gapbs::ops::ops(&graph);
            })
        });
    }};
}

macro_rules! bench_ops_epoch_mt {
    ($name: tt, $group: expr) => {{
        $group.bench_function($name, |b| {
            let mut builder = BuilderBase::new();
            let graph: gapbs::graphmodels::epoch::Graph<usize> = builder.make_graph();
            b.iter(|| {
                gapbs::ops::ops_epoch_mt(&graph);

            })
        });
    }};
}

macro_rules! bench_ops_mt {
    ($name: tt, $graphmodel: path, $group: expr) => {{
        use $graphmodel as graphmodel;
        $group.bench_function($name, |b| {
            b.iter(|| {
                let mut builder = BuilderBase::new();
                let graph: graphmodel::Graph<usize> = builder.make_graph();
                let mut source_picker = SourcePicker::new(&graph);
                benchmark_kernel(
                    &graph,
                    Box::new(|g| {
                        gapbs::ops::ops_mt(g);
                    }),
                    Box::new(|| {}),
                    Box::new(|| {}),
                );
            })
        });
    }};
}

macro_rules! bench_bfs {
    ($name: tt, $graphmodel: path, $group: expr) => {{
        use $graphmodel as graphmodel;
        $group.bench_function($name, |b| {
            let mut builder = BuilderBase::new();
            let graph: graphmodel::Graph<usize> = builder.make_graph();

            b.iter(|| {
                let mut source_picker = SourcePicker::new(&graph);
                benchmark_kernel_with_sp(
                    &graph,
                    &mut source_picker,
                    Box::new(|g, mut sp| {
                        gapbs::bfs::do_bfs(g, sp.pick_next());
                    }),
                    Box::new(|| {}),
                    Box::new(|| {}),
                );
            })
        });
    }};
}

macro_rules! bench_sssp {
    ($name: tt, $graphmodel: path, $group: expr) => {{
        use $graphmodel as graphmodel;
        $group.bench_function($name, |b| {
            let mut builder = BuilderBase::new();
            let graph: graphmodel::Graph<usize> = builder.make_graph();

            b.iter(|| {
                let mut source_picker = SourcePicker::new(&graph);
                benchmark_kernel_with_sp(
                    &graph,
                    &mut source_picker,
                    Box::new(|g, sp| {
                        gapbs::sssp::delta_step(g, sp.pick_next(), 1);
                    }),
                    Box::new(|| {}),
                    Box::new(|| {}),
                );
            })
        });
    }};
}

macro_rules! bench_sssp_mt {
    ($name: tt, $graphmodel: path, $group: expr) => {{
        use $graphmodel as graphmodel;
        $group.bench_function($name, |b| {
            let mut builder = BuilderBase::new();
            let graph: graphmodel::Graph<usize> = builder.make_graph();

            b.iter(|| {
                let mut source_picker = SourcePicker::new(&graph);
                benchmark_kernel_with_sp(
                    &graph,
                    &mut source_picker,
                    Box::new(|g, sp| {
                        gapbs::sssp::delta_step_mt(g, sp.pick_next(), 1);
                    }),
                    Box::new(|| {}),
                    Box::new(|| {}),
                );
            })
        });
    }};
}

macro_rules! bench_tc {
    ($name: tt, $graphmodel: path, $group: expr) => {{
        use $graphmodel as graphmodel;
        $group.bench_function($name, |b| {
            let mut builder = BuilderBase::new();
            let graph: graphmodel::Graph<usize> = builder.make_graph();
            let source_picker = SourcePicker::new(&graph);

            b.iter(|| {
                source_picker.benchmark_kernel_tc();
            })
        });
    }};
}

macro_rules! bench_tc_mt {
    ($name: tt, $graphmodel: path, $group: expr) => {{
        use $graphmodel as graphmodel;
        $group.bench_function($name, |b| {
            let mut builder = BuilderBase::new();
            let graph: graphmodel::Graph<usize> = builder.make_graph();
            let source_picker = SourcePicker::new(&graph);

            b.iter(|| {
                benchmark_kernel(
                    &graph,
                    Box::new(|g: &graphmodel::Graph<usize>| gapbs::tc::hybrid_mt(g)),
                    Box::new(|| {}),
                    Box::new(|| {}),
                );
            })
        });
    }};
}

macro_rules! bench_cc {
    ($name: tt, $graphmodel: path, $group: expr) => {{
        use $graphmodel as graphmodel;
        $group.bench_function($name, |b| {
            let mut builder = BuilderBase::new();
            let graph: graphmodel::Graph<usize> = builder.make_graph();
            let source_picker = SourcePicker::new(&graph);

            b.iter(|| {
                source_picker.benchmark_kernel_cc();
            })
        });
    }};
}

macro_rules! bench_cc_mt {
    ($name: tt, $graphmodel: path, $group: expr) => {{
        use $graphmodel as graphmodel;
        $group.bench_function($name, |b| {
            let mut builder = BuilderBase::new();
            let graph: graphmodel::Graph<usize> = builder.make_graph();
            let source_picker = SourcePicker::new(&graph);

            b.iter(|| {
                benchmark_kernel(
                    &graph,
                    Box::new(|g: &graphmodel::Graph<usize>| {
                        gapbs::cc::afforest_mt(g, None);
                    }),
                    Box::new(|| {}),
                    Box::new(|| {}),
                );
            })
        });
    }};
}

macro_rules! bench_pr {
    ($name: tt, $graphmodel: path, $group: expr) => {{
        use $graphmodel as graphmodel;
        $group.bench_function($name, |b| {
            let mut builder = BuilderBase::new();
            let graph: graphmodel::Graph<usize> = builder.make_graph();
            let source_picker = SourcePicker::new(&graph);

            b.iter(|| {
                source_picker.benchmark_kernel_pr();
            })
        });
    }};
}

macro_rules! bench_pr_mt {
    ($name: tt, $graphmodel: path, $group: expr) => {{
        use $graphmodel as graphmodel;
        $group.bench_function($name, |b| {
            let mut builder = BuilderBase::new();
            let graph: graphmodel::Graph<usize> = builder.make_graph();
            let source_picker = SourcePicker::new(&graph);

            b.iter(|| {
                benchmark_kernel(
                    &graph,
                    Box::new(|g: &graphmodel::Graph<usize>| {
                        gapbs::pr::page_rank_pull_mt(g, 20, Some(0.0004));
                    }),
                    Box::new(|| {}),
                    Box::new(|| {}),
                );
            })
        });
    }};
}

macro_rules! bench_bc {
    ($name: tt, $graphmodel: path, $group: expr) => {{
        use $graphmodel as graphmodel;
        $group.bench_function($name, |b| {
            let mut builder = BuilderBase::new();
            let graph: graphmodel::Graph<usize> = builder.make_graph();

            b.iter(|| {
                let mut source_picker = SourcePicker::new(&graph);
                benchmark_kernel_with_sp(
                    &graph,
                    &mut source_picker,
                    Box::new(|g: &graphmodel::Graph<usize>, mut sp| {
                        gapbs::bc::brandes(g, &mut sp, 1);
                    }),
                    Box::new(|| {}),
                    Box::new(|| {}),
                );
            })
        });
    }};
}

macro_rules! bench_bc_mt {
    ($name: tt, $graphmodel: path, $group: expr) => {{
        use $graphmodel as graphmodel;
        $group.bench_function($name, |b| {
            let mut builder = BuilderBase::new();
            let graph: graphmodel::Graph<usize> = builder.make_graph();

            b.iter(|| {
                let mut source_picker = SourcePicker::new(&graph);
                benchmark_kernel_with_sp(
                    &graph,
                    &mut source_picker,
                    Box::new(|g: &graphmodel::Graph<usize>, mut sp| {
                        gapbs::bc::brandes_mt(g, &mut sp, 1);
                    }),
                    Box::new(|| {}),
                    Box::new(|| {}),
                );
            })
        });
    }};
}

fn custom_criterion() -> Criterion {
    Criterion::default().sample_size(10)
}

// #[cfg(ops)]
// #[criterion(custom_criterion())]
// fn bench_generate(c: &mut Criterion) {
//     let mut group = c.benchmark_group("GENERATE");
//     bench_ops!("ARC", graphmodels::arc, group);
//     bench_ops!("RC", graphmodels::rc, group);
//     bench_ops!("CC", graphmodels::cc, group);
//     bench_ops!("GC", graphmodels::gc, group);
//     bench_ops!("ARENA", graphmodels::arena, group);
//     bench_ops!("EPOCH", graphmodels::epoch, group);
// }

#[cfg(feature = "ops")]
#[criterion(custom_criterion())]
fn bench_ops(c: &mut Criterion) {
    let mut group = c.benchmark_group("OPS");
    bench_ops!("EPOCH", graphmodels::epoch, group);
    bench_ops_epoch_mt!("EPOCH_mt_txn", group);
    bench_ops!("ARC", graphmodels::arc, group);
    // bench_ops_mt!("ARC_mt", graphmodels::arc, group);
    bench_ops!("RC", graphmodels::rc, group);
    bench_ops!("CC", graphmodels::cc, group);
    bench_ops!("GC", graphmodels::gc, group);
    bench_ops!("ARENA", graphmodels::arena, group);
}

#[cfg(feature = "bfs")]
#[criterion(custom_criterion())]
fn bench_bfs(c: &mut Criterion) {
    // let mut builder = BuilderBase::new();
    // let graph: graphmodels::rc::Graph<usize> = builder.make_graph();
    // graph.print_stats();
    let mut group = c.benchmark_group("BFS");
    bench_bfs!("ARC", graphmodels::arc, group);
    bench_bfs!("RC", graphmodels::rc, group);
    bench_bfs!("CC", graphmodels::cc, group);
    bench_bfs!("GC", graphmodels::gc, group);
    bench_bfs!("ARENA", graphmodels::arena, group);
    bench_bfs!("EPOCH", graphmodels::epoch, group);
}

#[cfg(feature = "sssp")]
#[criterion(custom_criterion())]
fn bench_sssp(c: &mut Criterion) {
    let mut group = c.benchmark_group("SSSP");
    bench_sssp!("ARC", graphmodels::arc, group);
    // bench_sssp_mt!("ARC_mt", graphmodels::arc, group);
    bench_sssp!("RC", graphmodels::rc, group);
    bench_sssp!("CC", graphmodels::cc, group);
    bench_sssp!("GC", graphmodels::gc, group);
    bench_sssp!("ARENA", graphmodels::arena, group);
    // bench_sssp_mt!("EPOCH_mt", graphmodels::epoch, group);
    bench_sssp!("EPOCH", graphmodels::epoch, group);
}

#[cfg(feature = "pr")]
#[criterion(custom_criterion())]
fn bench_pr(c: &mut Criterion) {
    let mut group = c.benchmark_group("PR");
    bench_pr!("ARC", graphmodels::arc, group);
    bench_pr_mt!("ARC_mt", graphmodels::arc, group);
    bench_pr!("RC", graphmodels::rc, group);
    bench_pr!("CC", graphmodels::cc, group);
    bench_pr!("GC", graphmodels::gc, group);
    bench_pr!("ARENA", graphmodels::arena, group);
    bench_pr!("EPOCH", graphmodels::epoch, group);
    bench_pr_mt!("EPOCH_mt", graphmodels::epoch, group);
}

#[cfg(feature = "cc")]
#[criterion(custom_criterion())]
fn bench_cc(c: &mut Criterion) {
    let mut group = c.benchmark_group("CC");
    bench_cc!("ARC", graphmodels::arc, group);
    bench_cc_mt!("ARC_mt", graphmodels::arc, group);
    bench_cc!("RC", graphmodels::rc, group);
    // bench_cc!("RC_btree", graphmodels::rcsorted, group);
    bench_cc!("CC", graphmodels::cc, group);
    bench_cc!("GC", graphmodels::gc, group);
    bench_cc!("ARENA", graphmodels::arena, group);
    bench_cc!("EPOCH", graphmodels::epoch, group);
    bench_cc_mt!("EPOCH_mt", graphmodels::epoch, group);
}

#[cfg(feature = "bc")]
#[criterion(custom_criterion())]
fn bench_bc(c: &mut Criterion) {
    let mut group = c.benchmark_group("BC");
    bench_bc!("ARC", graphmodels::arc, group);
    // bench_bc_mt!("ARC_mt", graphmodels::arc, group);
    bench_bc!("RC", graphmodels::rc, group);
    bench_bc!("CC", graphmodels::cc, group);
    bench_bc!("GC", graphmodels::gc, group);
    bench_bc!("ARENA", graphmodels::arena, group);
    bench_bc!("EPOCH", graphmodels::epoch, group);
    // bench_bc_mt!("EPOCH_mt", graphmodels::epoch, group);
}

#[cfg(feature = "tc")]
#[criterion(custom_criterion())]
fn bench_tc(c: &mut Criterion) {
    let mut group = c.benchmark_group("TC");
    bench_tc!("ARC", graphmodels::arc, group);
    bench_tc_mt!("ARC_mt", graphmodels::arc, group);
    bench_tc!("RC", graphmodels::rc, group);
    // // bench_tc!("RC_sorted", graphmodels::rcsorted, group);
    bench_tc!("CC", graphmodels::cc, group);
    bench_tc!("GC", graphmodels::gc, group);
    bench_tc!("ARENA", graphmodels::arena, group);
    bench_tc!("EPOCH", graphmodels::epoch, group);
    bench_tc_mt!("EPOCH_mt", graphmodels::epoch, group);
}
