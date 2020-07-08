# Memory Models for Graphs in Rust
Representing graphs in Rust is a problematic issue, as ownership
forbids typical representations found in e.g. C++.
A common approach is to use reference counting to represent graphs,
but this can easily lead to memory leaks if cycles
are present in the graph.
As naïve reference counting is not sufficient,
we must search for alternative representations.
In this repository, we explore different memory models that allow
safe representations of graph-like data structures in Rust.
These memory models are later evaluated in terms of performance and usability, by using the benchmarking system provided in `src/`.

We find that region-based allocation is, in most cases,
the best model to use when performance is of importance.
In cases where usability is more important, either reference-counting
with cycle collection or tracing garbage collection is a solid choice.
When it comes to multi-threading, we propose a new implementation
of a lock-free transactional graph in Rust. To our knowledge,
this is the first lock-free graph representation in Rust.
The model demonstrates poor scalability, but for certain graph topologies and sizes, it offers performance that exceeds the other graph models.


<img src="https://github.com/rasviitanen/rustgapbs/blob/master/reports/tc.svg">

## Memory Models
We have implemented six different graphs, located in `src/graphmodels`.

* Epoch - A lock free transactional graph that is `Send + Sync`. Uses epoch-based reclamation.
* Arc - A graph that is `Send + Sync`. Uses atomic reference counting.
* Arena - A graph that uses arena allocation.
* Cc - A graph that uses reference counting with a cycle collector to reclaim garbage cycles.
* Gc - A graph that uses tracing garbage collection to reclaim memory.
* Rc - A graph that uses reference counting.

## Benchmarks
We have ported [The GAP Benchmark Suite](https://github.com/sbeamer/gapbs), which can be run from the provided python-script.
To run a kernel, edit the provided python script.
You are also able to run a kernel directly via `cargo bench --features <kernel>`

In addition, you are able to run a custom benchmark called `OPS` that runs a custom distribution of operations.

We have also included a benchmark called `GC Bench`. This benchmark is available as a binary for each memory model.
See `src/bin` for available binaries. Make sure to run a release build when benchmarking.

The available datasets are provided by [The Koblenz Network Collection](http://konect.uni-koblenz.de/)
