#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused_variables)]
use gapbs::graphmodels::epoch::*;

use rayon::range;
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;
use std::thread;

use std::sync::atomic::{AtomicUsize, Ordering};

use crossbeam_epoch::{self as epoch, Atomic, Guard, Shared};


type PropType = (i32, i32);

struct Array {
    value: [f64; gapbs::types::kArraySize as usize],
}

fn TreeSize(i: i32) -> i32 {
    (1 << (i + 1)) - 1
}

fn NumIters(i: i32) -> i32 {
    2 * TreeSize(gapbs::types::kStretchTreeDepth) / TreeSize(i)
}

fn Populate<'a>(guard: &Guard, iDepth: i32, thisNode: Atomic<Node<'a, usize, EdgeInfo>>, graph: &'a Graph<'a, usize>) {
    if iDepth <= 0 {
        return;
    } else {
        let left = graph.add_vertex(1, None).expect("B1").1;
        let right = graph.add_vertex(1, None).expect("B2").1;

        let edge_info_l = EdgeInfo {
            node_id: 1,
            weight: None,
        };

        let edge_info_r = EdgeInfo {
            node_id: 2,
            weight: None,
        };

        unsafe {
            Graph::connect(
                thisNode.load(Ordering::SeqCst, guard).as_ref().unwrap(),
                edge_info_l,
                false,
            );

            Graph::connect(
                thisNode.load(Ordering::SeqCst, guard).as_ref().unwrap(),
                edge_info_r,
                false,
            );
        }

        Populate(guard, iDepth - 1, left, graph);
        Populate(guard, iDepth - 1, right, graph);
    }
}

fn MakeTree<'a>(guard: &Guard, iDepth: i32, graph: &'a Graph<'a, usize>) -> Atomic<Node<'a, usize, EdgeInfo>> {
    if iDepth <= 0 {
        graph.add_vertex(1, None).expect("B3").1
    } else {
        let left = MakeTree(guard, iDepth - 1, graph);
        let right = MakeTree(guard, iDepth - 1, graph);
        let result = graph.add_vertex(1, None).expect("B4").1;
        // Graph::connect(&result, EdgeInfo::new(left.clone()));
        // Graph::connect(&result, EdgeInfo::new(right.clone()));

        let edge_info_l = EdgeInfo {
            node_id: 1,
            weight: None,
        };

        let edge_info_r = EdgeInfo {
            node_id: 2,
            weight: None,
        };

        unsafe {
            Graph::connect(
                result.load(Ordering::SeqCst, guard).as_ref().unwrap(),
                edge_info_l,
                false,
            );

            Graph::connect(
                result.load(Ordering::SeqCst, guard).as_ref().unwrap(),
                edge_info_r,
                false,
            );
        }

        result
    }
}

// fn left_depth<'a>(depth: u64, n: Atomic<Node<'a, CustomNode<usize>, EdgeInfo>>) -> u64 {
//     let guard = &crossbeam_epoch::pin();
//     if let Some(left) = unsafe{n.node.as_ref()}.unwrap().list.as_ref().unwrap().iter(guard).next() {
//         if let Some(left_ref) = left.value().as_ref() {
//             left_depth(depth + 1, left_ref.vertex_ref.clone())
//         } else {
//             depth
//         }
//     } else {
//         depth
//     }
// }

// fn right_depth<'a>(depth: u64, n: Atomic<Node<'a, CustomNode<usize>, EdgeInfo>>) -> u64 {
//     let guard = &crossbeam_epoch::pin();
//     if let Some(right) = unsafe{n.node.as_ref()}.unwrap().list.as_ref().unwrap().iter(guard).skip(1).next() {
//         if let Some(right_ref) = right.value().as_ref() {
//             left_depth(depth + 1, right_ref.vertex_ref.clone())
//         } else {
//             depth
//         }
//     } else {
//         depth
//     }
// }

fn PrintDiagnostics() {}

fn TimeConstruction<'a>(depth: i32, graph: &'a Graph<'a, usize>) {
    let iNumIters = NumIters(depth);
    println!("creating {} trees of depth {}", iNumIters, depth);

    let tStart = time::now_utc();
    (0..iNumIters as u64).into_par_iter().for_each(|_| {
        let t_tree = Graph::new(gapbs::types::GRAPH_SIZE, false);
        let root = t_tree.add_vertex(1, None).expect("B").1;
        let guard = &epoch::pin();
        Populate(guard, depth, root, &t_tree);
        // destroy tempTree
    });

    let tFinish = time::now_utc();
    println!(
        "\tTop down construction took {} msec",
        (tFinish - tStart).num_milliseconds()
    );

    let tStart = time::now_utc();
    (0..iNumIters as u64).into_par_iter().for_each(|_| {
        let temp_graph = Graph::new(gapbs::types::GRAPH_SIZE, false);
        let guard = &epoch::pin();
        let tempTree = MakeTree(guard, depth, &temp_graph);
    });
    let tFinish = time::now_utc();
    println!(
        "\tButtom up construction took {} msec",
        (tFinish - tStart).num_milliseconds()
    );
}

pub fn main() {
    let guard = &epoch::pin();
    let tStart = time::now_utc();
    // Stretch the memory space quickly
    let temp_graph = Graph::new(gapbs::types::GRAPH_SIZE, false);
    let tempTree = MakeTree(guard, gapbs::types::kStretchTreeDepth, &temp_graph);
    // destroy tree

    // Create a long lived object
    println!(
        " Creating a long-lived binary tree of depth {}",
        gapbs::types::kLongLivedTreeDepth
    );
    let graph = Graph::new(gapbs::types::GRAPH_SIZE, false);
    let kLongLivedTree = graph.add_vertex(1, None).expect("B5").1;
    Populate(guard, gapbs::types::kLongLivedTreeDepth, kLongLivedTree.clone(), &graph);

    PrintDiagnostics();

    let mut d = gapbs::types::kMinTreeDepth;

    while d <= gapbs::types::kMaxTreeDepth {
        let time_construction_graph = Graph::new(gapbs::types::GRAPH_SIZE, false);
        TimeConstruction(d, &time_construction_graph);

        d += 2;
    }

    //    if array.array[1000] != 1.0f64 / (1000 as f64) {
    //        println!("Failed(array element wrong)");
    //    }

    // println!("Left depth: {:?}", left_depth(0, kLongLivedTree.clone()));
    // println!("Right depth: {:?}", right_depth(0, kLongLivedTree));

    let tFinish = time::now_utc();
    let tElapsed = (tFinish - tStart).num_milliseconds();

    PrintDiagnostics();
    println!("Completed in {} msec", tElapsed);
}
