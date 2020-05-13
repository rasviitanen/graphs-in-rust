#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused_variables)]
use gapbs::treenodes::TreeNode;
use generational_arena::{Arena, Index};

extern crate time;

use std::cell::RefCell;
use std::rc::Rc;

type SharedArena = Rc<RefCell<Arena<Node>>>;

struct Node {
    left: Option<Index>,
    right: Option<Index>,
}

impl Node {
    pub fn new(left: Option<Index>, right: Option<Index>, graph: &SharedArena) -> Index {
        let node = Node { left, right };
        graph.borrow_mut().insert(node)
    }
}

struct Array {
    value: [f64; gapbs::types::kArraySize as usize],
}

fn TreeSize(i: i32) -> i32 {
    (1 << (i + 1)) - 1
}

fn NumIters(i: i32) -> i32 {
    2 * TreeSize(gapbs::types::kStretchTreeDepth) / TreeSize(i)
}

fn Populate(iDepth: i32, thisNode: Index, graph: &SharedArena) {
    if iDepth <= 0 {
        return;
    } else {
        let left = Node::new(None, None, graph);
        let right = Node::new(None, None, graph);
        {
            let mut graph_borrow = graph.borrow_mut();
            let mut thisNode = graph_borrow.get_mut(thisNode).unwrap();

            (*thisNode).left = Some(left);
            (*thisNode).right = Some(right);
        }
        Populate(iDepth - 1, left, graph);
        Populate(iDepth - 1, right, graph);
    }
}

fn MakeTree(iDepth: i32, graph: &SharedArena) -> Index {
    if iDepth <= 0 {
        Node::new(None, None, graph)
    } else {
        let left = MakeTree(iDepth - 1, graph);
        let right = MakeTree(iDepth - 1, graph);
        let result = Node::new(Some(left), Some(right), graph);

        result
    }
}

// fn left_depth(depth: u64, n: Index) -> u64 {
//     if let Some(left) = n.read().unwrap().left() {
//         left_depth(depth + 1, left)
//     } else {
//         depth
//     }
// }

// fn right_depth(depth: u64, n: NodeModel::WrappedNode) -> u64 {
//     if let Some(right) = n.read().unwrap().right() {
//         right_depth(depth + 1, right)
//     } else {
//         depth
//     }
// }

fn PrintDiagnostics() {}

fn TimeConstruction(depth: i32) {
    let iNumIters = NumIters(depth);
    println!("creating {} trees of depth {}", iNumIters, depth);

    let tStart = time::now_utc();
    for _ in 0..iNumIters {
        let graph = Rc::new(RefCell::new(Arena::new()));
        let tempTree = Node::new(None, None, &graph);
        Populate(depth, tempTree, &graph);

        // destroy tempTree
    }
    let tFinish = time::now_utc();
    println!(
        "\tTop down construction took {} msec",
        (tFinish - tStart).num_milliseconds()
    );

    let tStart = time::now_utc();
    for _ in 0..iNumIters {
        let graph = Rc::new(RefCell::new(Arena::new()));
        let tempTree = MakeTree(depth, &graph);
    }
    let tFinish = time::now_utc();
    println!(
        "\tButtom up construction took {} msec",
        (tFinish - tStart).num_milliseconds()
    );
}

pub fn main() {
    let tStart = time::now_utc();

    // Stretch the memory space quickly
    let graph = Rc::new(RefCell::new(Arena::new()));
    let tempTree = MakeTree(gapbs::types::kStretchTreeDepth, &graph);
    // destroy tree

    // Create a long lived object
    println!(
        " Creating a long-lived binary tree of depth {}",
        gapbs::types::kLongLivedTreeDepth
    );

    let graph2 = Rc::new(RefCell::new(Arena::new()));
    let longLivedTree = Node::new(None, None, &graph2);
    Populate(
        gapbs::types::kLongLivedTreeDepth,
        longLivedTree.clone(),
        &graph2,
    );

    PrintDiagnostics();

    let mut d = gapbs::types::kMinTreeDepth;
    while d <= gapbs::types::kMaxTreeDepth {
        TimeConstruction(d);
        d += 2;
    }

    //    if array.array[1000] != 1.0f64 / (1000 as f64) {
    //        println!("Failed(array element wrong)");
    //    }

    // println!("Left depth: {:?}", left_depth(0, longLivedTree.clone()));
    // println!("Right depth: {:?}", right_depth(0, longLivedTree));

    let tFinish = time::now_utc();
    let tElapsed = (tFinish - tStart).num_milliseconds();

    PrintDiagnostics();
    println!("Completed in {} msec", tElapsed);
}
