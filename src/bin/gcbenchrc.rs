#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused_variables)]
use gapbs::treenodes::TreeNode;

extern crate time;

mod NodeModel {
    use gapbs::treenodes::rc;
    pub type Node = rc::Node;
    pub type WrappedNode = rc::WrappedNode;
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

fn Populate(iDepth: i32, thisNode: NodeModel::WrappedNode) {
    if iDepth <= 0 {
        return;
    } else {
        thisNode
            .borrow_mut()
            .set_edges(NodeModel::Node::new(), NodeModel::Node::new());
        Populate(iDepth - 1, thisNode.borrow().left().unwrap());
        Populate(iDepth - 1, thisNode.borrow().right().unwrap());
    }
}

fn MakeTree(iDepth: i32) -> NodeModel::WrappedNode {
    if iDepth <= 0 {
        NodeModel::Node::new()
    } else {
        let left = MakeTree(iDepth - 1);
        let right = MakeTree(iDepth - 1);
        let result = NodeModel::Node::new();
        result.borrow_mut().set_edges(left, right);

        result
    }
}

fn left_depth(depth: u64, n: NodeModel::WrappedNode) -> u64 {
    if let Some(left) = n.borrow().left() {
        left_depth(depth + 1, left)
    } else {
        depth
    }
}

fn right_depth(depth: u64, n: NodeModel::WrappedNode) -> u64 {
    if let Some(right) = n.borrow().right() {
        right_depth(depth + 1, right)
    } else {
        depth
    }
}

fn PrintDiagnostics() {}

fn TimeConstruction(depth: i32) {
    let iNumIters = NumIters(depth);
    println!("creating {} trees of depth {}", iNumIters, depth);

    let tStart = time::now_utc();
    for _ in 0..iNumIters {
        let tempTree = NodeModel::Node::new();
        Populate(depth, tempTree);

        // destroy tempTree
    }
    let tFinish = time::now_utc();
    println!(
        "\tTop down construction took {} msec",
        (tFinish - tStart).num_milliseconds()
    );

    let tStart = time::now_utc();
    for _ in 0..iNumIters {
        let tempTree = MakeTree(depth);
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
    let tempTree = MakeTree(gapbs::types::kStretchTreeDepth);
    // destroy tree

    // Create a long lived object
    println!(
        " Creating a long-lived binary tree of depth {}",
        gapbs::types::kLongLivedTreeDepth
    );
    let longLivedTree = NodeModel::Node::new();
    Populate(gapbs::types::kLongLivedTreeDepth, longLivedTree.clone());

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
