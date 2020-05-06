#![allow(dead_code)]

use crate::graphmodels::epoch::lftt::NodeDesc;
use epoch::{Atomic, Guard, Shared};

use std::sync::atomic::Ordering::SeqCst;

const F_ADP: usize = 0x1;
const F_DEL: usize = 0x2;
const F_ALL: usize = F_ADP | F_DEL;

#[inline]
fn set_adpinv(p: usize) -> usize {
    p | 1
}
#[inline]
fn clr_adpinv(p: usize) -> usize {
    p & !1
}
#[inline]
fn is_adpinv(p: usize) -> usize {
    p & 1
}

#[inline]
fn set_delinv(p: usize) -> usize {
    p | 2
}
#[inline]
fn clr_delinv(p: usize) -> usize {
    p & !2
}
#[inline]
fn is_delinv(p: usize) -> usize {
    p & 2
}

#[inline]
fn clr_invalid(p: usize) -> usize {
    p & !3
}
#[inline]
fn is_invalid(p: usize) -> usize {
    p & 3
}

pub enum LocatePredStatus {
    Found,
    LogicallyDeleted,
}

const DIMENSION: usize = 16;
const MASK: [usize; DIMENSION] = [
    0x3 << 30,
    0x3 << 28,
    0x3 << 26,
    0x3 << 24,
    0x3 << 22,
    0x3 << 20,
    0x3 << 18,
    0x3 << 16,
    0x3 << 14,
    0x3 << 12,
    0x3 << 10,
    0x3 << 8,
    0x3 << 6,
    0x3 << 4,
    0x3 << 2,
    0x3,
];

/// An entry in the adjacency list.
/// It is guaranteed to live as long as the Guard
/// that is used to get the entry.
pub struct Entry<'a: 'g, 'g, T: 'a, P> {
    pub node: &'g MDNode<'a, T, P>,
    _parent: &'a MDList<'a, T, P>,
    _guard: &'g Guard,
}

impl<'a: 'g, 'g, T: 'a, P> Entry<'a, 'g, T, P> {
    pub fn value(&self) -> Option<&T> {
        self.node.val.as_ref()
    }
}

pub struct Iter<'a: 'g, 'g, T: 'a, P: 'a> {
    parent: &'a MDList<'a, T, P>,
    head: Option<&'a MDNode<'a, T, P>>,
    guard: &'g Guard,
    stack: Vec<&'a Atomic<MDNode<'a, T, P>>>,
    dim: usize,
    pred_dim: usize,
}

impl<'a: 'g, 'g, T: 'a, P: 'a> Iterator for Iter<'a, 'g, T, P> {
    type Item = Entry<'a, 'g, T, P>;

    fn next(&mut self) -> Option<Entry<'a, 'g, T, P>> {
        unsafe {
            let guard = &*(self.guard as *const _);

            if self.dim != 0 {
                for d in self.dim..DIMENSION {
                    let child = &self.head.unwrap().children[d];
                    let loaded_child = child.load(SeqCst, self.guard);
                    if is_delinv(loaded_child.tag()) != 0 {
                        continue;
                    }
                    if let Some(child_ref) = loaded_child.as_ref() {
                        self.stack.push(&child);
                        if child_ref.val.is_some() {
                            self.dim = d + 1;
                            return Some(Entry {
                                node: child_ref,
                                _parent: self.parent,
                                _guard: self.guard,
                            });
                        }
                    }
                }
                self.dim = 0;
            }

            if self.head.is_none() {
                self.stack.push(&self.parent.head);
            }

            while let Some(node) = self.stack.pop().map(|n| n.load(SeqCst, guard)) {
                if node.is_null() || is_delinv(node.tag()) != 0 {
                    continue;
                }

                let node = node.as_ref().unwrap();

                // // The root node might not be logically added,
                // // so if it has no value, we skip it
                // if node.val.is_some() {
                //     return Some(Entry {
                //         node: node,
                //         _parent: self.parent,
                //         _guard: self.guard,
                //     });
                // }

                for d in 0..DIMENSION {
                    let child = &node.children[d];
                    let loaded_child = child.load(SeqCst, self.guard);
                    if is_delinv(loaded_child.tag()) != 0 {
                        continue;
                    }
                    if let Some(child_ref) = loaded_child.as_ref() {
                        self.stack.push(&child);
                        if child_ref.val.is_some() {
                            self.dim = d + 1;
                            self.head = Some(node);
                            return Some(Entry {
                                node: child_ref,
                                _parent: self.parent,
                                _guard: self.guard,
                            });
                        }
                    }
                }
            }
        }
        None

        // let mut next = None;
        // match self.head {
        //     Some(ref mut curr) => {
        //         unsafe {
        //             let (dim, found_next) = MDList::next_node(curr, self.dim, &mut self.stack, self.guard);
        //             if dim < DIMENSION {
        //                 dbg!(dim);
        //                 self.dim = dim + 1;
        //             } else {
        //                 self.dim = 0;
        //                 self.head = self.stack.pop();
        //             }
        //             next = found_next;
        //         }
        //     },
        //     None => {
        //         let guard = unsafe{&*(self.guard as *const _)};
        //         let curr = &mut self.parent.head.load(SeqCst, guard);
        //         unsafe {
        //             let (dim, found_next) = MDList::next_node(curr, self.dim, &mut self.stack, self.guard);
        //             dbg!(dim);
        //             if dim < DIMENSION {
        //                 dbg!("ITERATING OVER HEAD");
        //                 self.dim = dim + 1;
        //             } else {
        //                 self.dim = 0;
        //                 self.head = self.stack.pop();
        //             }
        //             next = found_next;
        //         }
        //     }
        // };

        // next.map(|n| Entry {
        //     node: unsafe{ n.as_ref().unwrap() },
        //     _parent: self.parent,
        //     _guard: self.guard,
        // })
    }
}

#[repr(C)]
pub struct MDDesc<'a, T, P> {
    dim: usize,
    pred_dim: usize,
    curr: Atomic<MDNode<'a, T, P>>,
}

/// A node in the `MDList`
/// Marked `repr(C)` to improve cache locality
#[repr(C)]
pub struct MDNode<'a, T, P> {
    pub key: usize,
    coord: [usize; DIMENSION],
    val: Option<T>,
    pub pending: Atomic<MDDesc<'a, T, P>>,
    pub node_desc: Atomic<NodeDesc<'a, P, T>>,
    pub children: [Atomic<Self>; DIMENSION],
}

impl<'a, T, P> MDNode<'a, T, P> {
    pub fn new(key: usize, val: Option<T>) -> Self {
        Self {
            key,
            val,
            coord: MDList::<T, P>::key_to_coord(key),
            pending: Atomic::null(),
            node_desc: Atomic::null(),
            children: Default::default(),
        }
    }
}

#[repr(C)]
pub struct MDList<'a, T, P> {
    basis: usize,
    head: Atomic<MDNode<'a, T, P>>,
}

impl<'a: 'd + 'g, 'd, 'g, T: 'a, P: 'a> MDList<'a, T, P> {
    pub fn new(basis: usize) -> Self {
        Self {
            head: Atomic::new(MDNode::new(0, None)),
            basis,
        }
    }

    pub fn iter(&'a self, guard: &'g Guard) -> Iter<'a, 'g, T, P> {
        Iter {
            parent: self,
            head: None,
            stack: Vec::new(),
            guard,
            dim: 0,
            pred_dim: 0,
        }
    }

    pub fn head(&self) -> &Atomic<MDNode<'a, T, P>> {
        &self.head
    }

    pub unsafe fn get(
        &'a self,
        key: usize,
        guard: &'g Guard,
    ) -> Result<Entry<'a, 'g, T, P>, impl std::fmt::Debug> {
        // Rebind lifetime to self
        let guard = &*(guard as *const _);

        let coord = Self::key_to_coord(key);
        let pred = &mut Shared::null();
        let curr = &mut self.head.load(SeqCst, guard);
        let mut dim = 0;
        let mut pred_dim = 0;
        if let LocatePredStatus::Found =
            Self::locate_pred(&coord, pred, curr, &mut dim, &mut pred_dim, guard)
        {
            if dim == DIMENSION {
                if let Some(curr_ref) = curr.as_ref() {
                    Ok(Entry {
                        node: curr_ref,
                        _parent: self,
                        _guard: guard,
                    })
                } else {
                    Err("Node was found, but it was NULL")
                }
            } else {
                Err("Node not found")
            }
        } else {
            Err("Node was found, but was logically deleted")
        }
    }

    pub fn entries(&'a self, guard: &'g Guard) -> Vec<Entry<'a, 'g, T, P>> {
        unsafe {
            let mut stack = Vec::new();
            stack.push(&self.head);

            let mut entries = Vec::new();
            while let Some(node) = stack.pop().map(|n| n.load(SeqCst, guard)) {
                if node.is_null() || is_delinv(node.tag()) != 0 {
                    continue;
                }

                let node = node.as_ref().unwrap();

                // The root node might not be logically added,
                // so if it has no value, we skip it
                if node.val.is_some() {
                    entries.push(Entry {
                        node,
                        _parent: self,
                        _guard: guard,
                    })
                }

                for d in 0..DIMENSION {
                    let child = &node.children[d];
                    if !child.load(SeqCst, guard).is_null() {
                        stack.push(&child);
                    }
                }
            }

            entries
        }
    }

    pub unsafe fn insert(
        &self,
        new_node: &Atomic<MDNode<'a, T, P>>,
        pred: &mut Shared<'a, MDNode<'a, T, P>>,
        curr: &mut Shared<'a, MDNode<'a, T, P>>,
        dim: &mut usize,
        pred_dim: &mut usize,
        guard: &Guard,
    ) -> bool {
        let guard = &*(guard as *const _);

        let pred_ref = pred.as_ref().unwrap(); // Safe unwrap
        let pred_child = &pred_ref.children[*pred_dim].load(SeqCst, guard);
        if *dim == DIMENSION && is_delinv(pred_child.tag()) == 0 {
            return false;
        }

        let expected;
        if is_delinv(pred_child.tag()) == 0 {
            expected = *curr;
        } else {
            expected = curr.with_tag(set_delinv(curr.tag()));

            if *dim == DIMENSION - 1 {
                *dim = DIMENSION;
            }
        }

        if *pred_child == expected {
            let desc = Self::fill_new_node(
                &new_node.load(SeqCst, guard).as_ref().unwrap(),
                pred,
                expected,
                dim,
                pred_dim,
                guard,
            );

            if let Ok(pred_child) = pred_ref.children[*pred_dim].compare_and_set(
                expected,
                new_node.load(SeqCst, guard),
                SeqCst,
                guard,
            ) {
                if pred_child == expected && !desc.load(SeqCst, guard).is_null() {
                    if let Some(curr_ref) = curr.as_ref() {
                        Self::finish_inserting(
                            curr_ref,
                            curr_ref.pending.load(SeqCst, guard),
                            guard,
                        );
                    }

                    Self::finish_inserting(
                        &new_node.load(SeqCst, guard).as_ref().unwrap(),
                        desc.load(SeqCst, guard),
                        guard,
                    );
                }
            }
            return true;
        }

        //If the code reaches here it means the CAS failed
        //Three reasons why CAS may fail:
        //1. the child slot has been marked as invalid by parents
        //2. another thread inserted a child into the slot
        //3. another thread deleted the child
        if is_adpinv(pred_child.tag()) != 0 {
            *pred = Shared::null();
            *curr = self.head.load(SeqCst, guard);
            *dim = 0;
            *pred_dim = 0;
        } else if pred_child.with_tag(clr_invalid(pred_child.tag())) == *curr {
            *curr = *pred;
            *dim = *pred_dim;
        }

        if let Some(new_node_ref) = new_node.load(SeqCst, guard).as_ref() {
            if !new_node_ref.pending.load(SeqCst, guard).is_null() {
                new_node_ref.pending.store(Shared::null(), SeqCst);
            }
        }

        false
    }

    pub unsafe fn delete<'t>(
        pred: &mut Shared<'t, MDNode<'a, T, P>>,
        curr: &mut Shared<'t, MDNode<'a, T, P>>,
        pred_dim: &mut usize,
        dim: &mut usize,
        guard: &Guard,
    ) -> bool {
        if *dim == DIMENSION {
            let pred_child = &pred.as_ref().unwrap().children[*pred_dim];

            if pred_child.load(SeqCst, guard) == *curr
                && pred_child
                    .compare_and_set(*curr, curr.with_tag(set_delinv(curr.tag())), SeqCst, guard)
                    .is_ok()
            {
                return true;
            }
        }

        // Node deos not exist, or it is marked by another thread
        false
    }

    pub unsafe fn find(&self, key: usize, guard: &Guard) -> bool {
        // Rebind lifetime to self
        let guard = &*(guard as *const _);

        let coord = Self::key_to_coord(key);
        let pred = &mut Shared::null();
        let curr = &mut self.head.load(SeqCst, guard);
        let mut dim = 0;
        let mut pred_dim = 0;

        Self::locate_pred(&coord, pred, curr, &mut dim, &mut pred_dim, guard);

        dim == DIMENSION
    }

    /// Computes the 16th root of a given key
    #[inline]
    pub fn key_to_coord(key: usize) -> [usize; DIMENSION] {
        // let mut coords = [0; DIMENSION];
        // for i in 0..DIMENSION {
        //     coords[i] = (key & MASK[i]) >> (30 - (i << 1));
        // }
        // coords

        // The above code is 83 assebly instructions, this is 63.
        // We mainly abvoid movups instructions
        // Does it improve performance signficantly? ¯\_(ツ)_/¯, probably not
        [
            (key & MASK[0]) >> 30,
            (key & MASK[1]) >> (30 - (1 << 1)),
            (key & MASK[2]) >> (30 - (2 << 1)),
            (key & MASK[3]) >> (30 - (3 << 1)),
            (key & MASK[4]) >> (30 - (4 << 1)),
            (key & MASK[5]) >> (30 - (5 << 1)),
            (key & MASK[6]) >> (30 - (6 << 1)),
            (key & MASK[7]) >> (30 - (7 << 1)),
            (key & MASK[8]) >> (30 - (8 << 1)),
            (key & MASK[9]) >> (30 - (9 << 1)),
            (key & MASK[10]) >> (30 - (10 << 1)),
            (key & MASK[11]) >> (30 - (11 << 1)),
            (key & MASK[12]) >> (30 - (12 << 1)),
            (key & MASK[13]) >> (30 - (13 << 1)),
            (key & MASK[14]) >> (30 - (14 << 1)),
            (key & MASK[15]),
        ]
    }

    #[inline]
    pub unsafe fn next_node(
        curr: &mut Shared<'a, MDNode<'a, T, P>>,
        dim: usize,
        stack: &mut Vec<Shared<'a, MDNode<'a, T, P>>>,
        guard: &Guard,
    ) -> (usize, Option<Shared<'a, MDNode<'a, T, P>>>) {
        let guard = &*(guard as *const _);
        stack.push(*curr);

        // while let Some(node) = stack.pop() {
        // if node.is_null() || is_delinv(node.tag()) != 0 {
        //     continue;
        // }

        let node_ref = curr.as_ref().unwrap();

        for d in dim..DIMENSION {
            let child = &node_ref.children[d];
            let loaded_child = child.load(SeqCst, guard);

            if let Some(child_ref) = loaded_child.as_ref() {
                dbg!("a");
                stack.push(loaded_child);
                if child_ref.val.is_some() {
                    dbg!("FOUND");
                    return (d, Some(*curr));
                }
            }
        }
        // }

        dbg!("NO CHILD");
        (dim, None)
    }

    #[inline]
    pub unsafe fn locate_pred<'t>(
        coord: &[usize; DIMENSION],
        pred: &mut Shared<'t, MDNode<'a, T, P>>,
        curr: &mut Shared<'t, MDNode<'a, T, P>>,
        dim: &mut usize,
        pred_dim: &mut usize,
        guard: &Guard,
    ) -> LocatePredStatus
    where
        'a: 't,
    {
        let guard = &*(guard as *const _);
        let mut status = LocatePredStatus::Found;
        // Locate the proper position to insert
        // tranverses list from low dim to high dim
        while *dim < DIMENSION {
            // Locate predecessor and successor
            while let Some(curr_ref) = curr.as_ref() {
                if coord[*dim] > curr_ref.coord[*dim] {
                    *pred_dim = *dim;
                    *pred = *curr;

                    let pending = curr_ref.pending.load(SeqCst, guard);
                    if let Some(pending_ref) = pending.as_ref() {
                        if *dim >= pending_ref.pred_dim && *dim <= pending_ref.dim {
                            Self::finish_inserting(&curr_ref, pending, guard);
                        }
                    }

                    let child = curr_ref.children[*dim].load(SeqCst, guard);
                    if is_delinv(child.tag()) != 0 {
                        status = LocatePredStatus::LogicallyDeleted;
                    };
                    *curr = child.with_tag(clr_invalid(child.tag()));
                } else {
                    break;
                }
            }

            // No successor has greater coord at this dimension
            // The position after pred is the insertion position
            if let Some(curr_ref) = curr.as_ref() {
                if coord[*dim] < curr_ref.coord[*dim] {
                    break;
                }
            } else {
                break;
            }
            *dim += 1;
        }

        status
    }

    #[inline]
    fn fill_new_node(
        new_node: &MDNode<'a, T, P>,
        _pred: &mut Shared<'a, MDNode<'a, T, P>>,
        curr: Shared<'a, MDNode<'a, T, P>>,
        dim: &mut usize,
        pred_dim: &mut usize,
        guard: &Guard,
    ) -> Atomic<MDDesc<'a, T, P>> {
        let mut desc = Atomic::null();
        if *pred_dim != *dim {
            let curr_untagged = Atomic::null();
            curr_untagged.store(curr.with_tag(clr_delinv(curr.tag())), SeqCst);
            desc = Atomic::new(MDDesc {
                curr: curr_untagged,
                pred_dim: *pred_dim,
                dim: *dim,
            });
        }

        //Fill values for new_node, m_child is set to 1 for all children before pred_dim
        //pred_dim is the dimension where new_node is inserted, all dimension before that are invalid for new_node
        for i in 0..*pred_dim {
            new_node.children[i].store(
                new_node.children[i].load(SeqCst, guard).with_tag(0x1),
                SeqCst,
            );
        }

        // FIXME:(rasmus) A memset is missing here...
        if *dim < DIMENSION {
            new_node.children[*dim].store(curr, SeqCst);
        }

        new_node.pending.store(desc.load(SeqCst, guard), SeqCst);

        desc
    }

    #[inline]
    pub unsafe fn finish_inserting(
        n: &MDNode<'a, T, P>,
        desc: Shared<'a, MDDesc<'a, T, P>>,
        guard: &Guard,
    ) {
        let desc_ref = desc.as_ref().unwrap(); // Safe unwrap
        let pred_dim = desc_ref.pred_dim;
        let dim = desc_ref.dim;
        let curr = &desc_ref.curr;

        for i in pred_dim..dim {
            let child = &curr.load(SeqCst, guard).as_ref().unwrap().children[i];
            let g_child = child.fetch_or(0x1, SeqCst, guard);
            let g_child = g_child.with_tag(clr_adpinv(g_child.tag()));

            if !g_child.is_null() {
                // Adopt children form curr_node's
                // FIXME:(rasmus) Unnecessary load? CAS only succeeds if it is null...
                if n.children[i].load(SeqCst, guard).is_null()
                    && n.children[i]
                        .compare_and_set(Shared::null(), g_child, SeqCst, guard)
                        .is_err()
                {}
            }
        }

        if n.pending.load(SeqCst, guard) == desc {
            if let Ok(_p) = n
                .pending
                .compare_and_set(desc, Shared::null(), SeqCst, guard)
            {
                // FIXME:(rasmus) Do proper clean-up here
                // guard.defer_destroy(p);
            }
        }
    }
}
