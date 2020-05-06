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
        &mut source_picker2,
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
        &mut source_picker3,
        Box::new(|g: &Graph, mut sp| {
            gapbs::bc::brandes(g, &mut sp, 1);
        }),
        Box::new(|| {}),
        Box::new(|| {}),
    );
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
                    Box::new(|g, mut sp| {
                        gapbs::sssp::delta_step(g, sp.pick_next(), 1);
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

fn custom_criterion() -> Criterion {
    Criterion::default().sample_size(10)
}

#[criterion(custom_criterion())]
fn bench_bfs(c: &mut Criterion) {
    let mut group = c.benchmark_group("BFS");
    bench_bfs!("ARC", graphmodels::arc, group);
    bench_bfs!("RC", graphmodels::rc, group);
    bench_bfs!("CC", graphmodels::cc, group);
    bench_bfs!("GC", graphmodels::gc, group);
    bench_bfs!("ARENA", graphmodels::arena, group);
}

#[criterion(custom_criterion())]
fn bench_sssp(c: &mut Criterion) {
    let mut group = c.benchmark_group("SSSP");
    bench_sssp!("ARC", graphmodels::arc, group);
    bench_sssp!("RC", graphmodels::rc, group);
    bench_sssp!("CC", graphmodels::cc, group);
    bench_sssp!("GC", graphmodels::gc, group);
    bench_sssp!("ARENA", graphmodels::arena, group);
}

#[criterion(custom_criterion())]
fn bench_pr(c: &mut Criterion) {
    let mut group = c.benchmark_group("PR");
    bench_pr!("ARC", graphmodels::arc, group);
    bench_pr!("RC", graphmodels::rc, group);
    bench_pr!("CC", graphmodels::cc, group);
    bench_pr!("GC", graphmodels::gc, group);
    bench_pr!("ARENA", graphmodels::arena, group);
}

#[criterion(custom_criterion())]
fn bench_cc(c: &mut Criterion) {
    let mut group = c.benchmark_group("CC");
    bench_cc!("ARC", graphmodels::arc, group);
    bench_cc!("RC", graphmodels::rc, group);
    bench_cc!("CC", graphmodels::cc, group);
    bench_cc!("GC", graphmodels::gc, group);
    bench_cc!("ARENA", graphmodels::arena, group);
}

#[criterion(custom_criterion())]
fn bench_bc(c: &mut Criterion) {
    let mut group = c.benchmark_group("BC");
    bench_bc!("ARC", graphmodels::arc, group);
    bench_bc!("RC", graphmodels::rc, group);
    bench_bc!("CC", graphmodels::cc, group);
    bench_bc!("GC", graphmodels::gc, group);
    bench_bc!("ARENA", graphmodels::arena, group);
}

#[criterion(custom_criterion())]
fn bench_tc(c: &mut Criterion) {
    let mut group = c.benchmark_group("TC");
    bench_tc!("ARC", graphmodels::arc, group);
    bench_tc!("RC", graphmodels::rc, group);
    bench_tc!("CC", graphmodels::cc, group);
    bench_tc!("GC", graphmodels::gc, group);
    bench_tc!("ARENA", graphmodels::arena, group);
}
