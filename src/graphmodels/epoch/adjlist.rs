#![allow(dead_code)]
use crate::graphmodels::epoch::lftt::{Desc, NodeDesc, OpStatus, OpType, Operator, ReturnCode};
use crate::graphmodels::epoch::mdlist::{MDList, MDNode};
use epoch::{Atomic, Guard, Owned, Shared};

// use bloom::{BloomFilter, ASMS};
use lock_free_bloomfilter::bloomfilter::BloomFilter;
use std::cell::RefCell;
use std::sync::atomic::Ordering::{Relaxed, SeqCst};
use std::sync::{Arc, RwLock};
use utils::atomic::AtomicCell;

const KEY_RANGE: usize = 1024;
const BASIS: usize = 16;
thread_local!(static HELPSTACK: RefCell<Vec<*const u8>> = RefCell::new(Vec::new()));

#[inline]
fn set_mark(p: usize) -> usize {
    p | 1
}
#[inline]
fn clr_mark(p: usize) -> usize {
    p & !1
}
#[inline]
fn clr_markd(p: usize) -> usize {
    p & !1
}
#[inline]
fn is_marked(p: usize) -> usize {
    p & 1
}
#[inline]
fn is_delinv(p: usize) -> usize {
    p & 2
}
#[inline]
fn set_delinv(p: usize) -> usize {
    p | 2
}

/// Marked `repr(C)` to improve cache locality,
/// as the dynamically sized list will be placed last,
/// making the other fields be closer
#[repr(C)]
pub struct Node<'a, T, E> {
    pub key: usize,
    value: AtomicCell<Option<T>>,
    node_desc: Atomic<NodeDesc<'a, T, E>>,
    next: Atomic<Self>,
    pub list: Option<MDList<'a, E, T>>,
}

impl<'a, T: Copy, E> Node<'a, T, E> {
    pub fn value(&self) -> Option<T> {
        self.value.load()
    }
}

impl<'a, T, E> Node<'a, T, E> {
    #[inline]
    fn new(
        key: usize,
        value: Option<T>,
        next: Atomic<Self>,
        node_desc: Atomic<NodeDesc<'a, T, E>>,
        list: Option<MDList<'a, E, T>>,
    ) -> Self {
        Self {
            key,
            value: AtomicCell::new(value),
            next,
            node_desc,
            list,
        }
    }
}

unsafe impl<'a, T: Send + Sync, E: Send + Sync> Send for AdjacencyList<'a, T, E> {}
unsafe impl<'a, T: Send + Sync, E: Send + Sync> Sync for AdjacencyList<'a, T, E> {}

#[derive(Clone)]
pub struct RefEntry<'a: 't, 't, T: 'a, E: 'a> {
    pub node: Shared<'t, Node<'a, T, E>>,
}

impl<'a: 't, 't, T: 'a, E: 'a> RefEntry<'a, 't,  T, E> {
    #[must_use]
    pub fn get(&self) -> &Node<'a, T, E> {
        unsafe { self.node.as_ref().expect("Refentry was NULL") }
    }
}

impl<'a, 't, T, E> std::ops::Deref for RefEntry<'a, 't, T, E> {
    type Target = Node<'a, T, E>;

    fn deref(&self) -> &Self::Target {
        unsafe { self.node.as_ref().expect("Refentry was NULL") }
    }
}

pub struct IterRefEntry<'a: 't + 'g, 't, 'g, T: 'a, E: 'a> {
    parent: &'a AdjacencyList<'a, T, E>,
    guard: &'g Guard,
    head: Option<RefEntry<'a, 't, T, E>>,
}

impl<'a: 't + 'g, 't, 'g, T: 'a + Clone, E: 'a + Clone> Iterator for IterRefEntry<'a, 't, 'g, T, E> {
    type Item = RefEntry<'a, 't, T, E>;

    fn next(&mut self) -> Option<Self::Item> {
        let guard = unsafe { &*(self.guard as *const _) };

        if let Some(head) = self.head.as_ref() {
            if head.node.is_null() {
                return None;
            }
            let next = RefEntry {
                node: unsafe { head.node.as_ref() }
                    .unwrap()
                    .next
                    .load(SeqCst, guard),
            };
            self.head.replace(next);
            self.head.as_ref().map(RefEntry::clone)
        } else {
            self.head.replace(RefEntry {
                node: self.parent.head.load(SeqCst, guard),
            });
            self.head.as_ref().map(RefEntry::clone)
        }
    }
}

pub struct AdjacencyList<'a, T, E> {
    head: Atomic<Node<'a, T, E>>,
    cursor: Atomic<Node<'a, T, E>>,
    tail: Atomic<Node<'a, T, E>>,
    // bloom_filter: BloomFilter,
}

/// Uh... FIXME:(rasmus)
unsafe impl<#[may_dangle] 'a, #[may_dangle] T, #[may_dangle] E> Drop for AdjacencyList<'a, T, E> {
    fn drop(&mut self) {
        let guard = &epoch::pin();
        unsafe {
            let mut prev = self.head.load(SeqCst, guard);
            loop {
                if prev.is_null() {
                    break;
                }
                let next = prev.as_ref().unwrap().next.load(SeqCst, guard);
                guard.defer_destroy(prev);
                prev = next;
            }
        }
    }
}

impl<'a: 'd + 'g, 'd, 'g, T: 'a + Clone, E: 'a + Clone> AdjacencyList<'a, T, E> {
    // Public operations
    pub fn new(size_hint: i64) -> Self {
        let guard = &epoch::pin();
        let head = Node::new(0, None, Atomic::null(), Atomic::null(), None);
        let tail = Atomic::new(Node::new(
            usize::max_value(),
            None,
            Atomic::null(),
            Atomic::null(),
            None,
        ));
        head.next.store(tail.load(SeqCst, guard), SeqCst);

        let head = Atomic::new(head);
        AdjacencyList {
            cursor: head.clone(),
            head,
            tail,
            // bloom_filter: BloomFilter::create(size_hint, 0.0001),
        }
    }

    pub fn iter<'t>(&'a self, guard: &'g Guard) -> IterRefEntry<'a, 't, 'g, T, E> {
        IterRefEntry {
            parent: self,
            guard,
            head: None,
        }
    }

    pub fn execute_ops<'t>(
        &'t self,
        desc: *const Desc<'a, T, E>,
        sender: std::sync::mpsc::Sender<ReturnCode<RefEntry<'a, 't, T, E>>>,
        guard: &'g Guard,
    ) where
        'a: 't,
    {
        HELPSTACK.with(|hs| {
            hs.replace(Vec::new());
        });

        unsafe {
            self.help_ops(desc, 0, &Some(sender), guard);
        }
        // // Check execution status
        // let op_status = desc
        //     .status
        //     .load();

        // op_status
    }

    // Internal operations

    /// Inserts a vertex to the adjacency list
    ///
    /// # Safety
    ///
    /// Should not be called directly
    #[inline]
    pub unsafe fn insert_vertex<'t>(
        &'t self,
        vertex: usize,
        value: &Option<T>,
        desc: *const Desc<'a, T, E>,
        opid: usize,
        inserted: &mut Shared<'t, Node<'a, T, E>>,
        pred: &mut Shared<'t, Node<'a, T, E>>,
        guard: &Guard,
    ) -> ReturnCode<RefEntry<'a, 't, T, E>>
    where
        'a: 't,
    {
        let guard = &*(guard as *const _);
        *inserted = Shared::null();
        let cursor = self.cursor.load_consume(guard);

        let current = &mut self.head.load_consume(guard);

        let n_desc = Atomic::new(NodeDesc::new(desc, opid));
        loop {
            // if self.bloom_filter.might_contain(vertex) {
            //     self.locate_pred(pred, current, vertex, guard);
            // } else {
            // If the node is definitely not in the list, we skip the location on pred,
            // and just append the vertex to the end of the list.
            *pred = cursor;
            *current = cursor.as_ref().unwrap().next.load_consume(guard);
            // }

            // Check if node is physically in the list
            if Self::is_node_exist(*current, vertex) {
                // If the node is physically in the list, it may be possible to simply update the descriptor
                let current_ref = &current.as_ref().unwrap();
                let current_desc = &current_ref.node_desc;

                //Check if node descriptor is marked for deletion
                //If it has, we cannot update the descriptor and must perform physical removal
                let g_current_desc = current_desc.load(SeqCst, guard);
                if is_marked(g_current_desc.tag()) != 0 {
                    if is_marked(current_ref.next.load(SeqCst, epoch::unprotected()).tag()) == 0 {
                        current_ref.next.fetch_or(0x1, SeqCst, epoch::unprotected());
                    }
                    *current = self.head.load_consume(guard);
                    continue;
                }

                self.finish_pending_txn(g_current_desc, desc, guard);

                if Self::is_same_operation(
                    g_current_desc.as_ref().unwrap(),
                    // We are the only one accessing n_desc...
                    n_desc.load(Relaxed, epoch::unprotected()).as_ref().unwrap(),
                ) {
                    return ReturnCode::Skip;
                }

                // Check is node is logically in the list
                if Self::is_key_exist(g_current_desc.as_ref().unwrap(), guard) {
                    // The Node is in the list, but it is not certain that it has the new value.
                    // For this reason, we update the Node.
                    // FIXME:(rasmus) This returns Fail in the original code...
                    current.as_ref().unwrap().value.store(value.clone());
                    return ReturnCode::Success;
                } else {
                    match (*desc).status.load() {
                        OpStatus::Active => {}
                        _ => return ReturnCode::Fail("Transaction is inactive".into()),
                    }

                    if current
                        .as_ref()
                        .unwrap()
                        .node_desc
                        .compare_and_set(
                            g_current_desc,
                            n_desc.load(Relaxed, epoch::unprotected()),
                            SeqCst,
                            guard,
                        )
                        .is_ok()
                    {
                        *inserted = *current;
                        return ReturnCode::Inserted(RefEntry { node: *inserted });
                    }
                }
            } else {
                if let OpStatus::Active = (*desc).status.load() {
                } else {
                    return ReturnCode::Fail("Transaction is inactive".into());
                }

                let mut new_node = None;
                if new_node.is_none() {
                    let mdlist = MDList::new(KEY_RANGE);

                    mdlist.head().load(SeqCst, guard).deref_mut().node_desc = n_desc.clone();

                    new_node.replace(Node::new(
                        vertex,
                        value.clone(),
                        Atomic::null(),
                        n_desc.clone(),
                        Some(mdlist),
                    ));
                }

                new_node.as_ref().unwrap().next.store(*current, Relaxed);

                let next = &pred.as_ref().unwrap().next;
                if let Ok(p) =
                    next.compare_and_set(*current, Owned::new(new_node.unwrap()), SeqCst, guard)
                {
                    *inserted = p;
                    self.cursor.store(*inserted, Relaxed);
                    // self.bloom_filter.set(vertex);
                    return ReturnCode::Inserted(RefEntry { node: *inserted });
                }

                *current = if is_marked(next.load(SeqCst, epoch::unprotected()).tag()) == 0 {
                    *pred
                } else {
                    self.head.load(SeqCst, guard)
                };
            }
        }
    }

    /// Connects two nodes
    ///
    /// # Safety
    ///
    /// Should not be called directly?
    #[inline]
    pub unsafe fn connect<'t>(
        vertex_node: &RefEntry<'a, 't, T, E>,
        edge: usize,
        edge_node: E,
    ) -> ReturnCode<RefEntry<'a, 't, T, E>> {
        let dim = &mut 0;
        let pred_dim = &mut 0;
        let guard = &*(&epoch::pin() as *const _);
        let inserted: &mut Shared<'a, MDNode<'a, E, T>> = &mut Shared::null();
        let md_pred: &mut Shared<'a, MDNode<'a, E, T>> = &mut Shared::null();

        // Try to find the vertex to which the current key is adjacenct,
        // if it is not found, we check if the vertex and edge are the same vertex.
        let current_ref = vertex_node.get();
        let mdlist = &current_ref.list.as_ref().expect("NO MD LIST");
        let md_current = &mut mdlist.head().load(SeqCst, guard);

        let new_md_node = MDNode::new(edge, Some(edge_node));
        let new_node = Atomic::new(new_md_node);

        let coord = MDList::<E, T>::key_to_coord(edge);
        MDList::locate_pred(&coord, md_pred, md_current, dim, pred_dim, guard);
        let md_pred_ref = md_pred.as_ref().expect("MDPred was NULL");
        let pred_child = md_pred_ref.children[*pred_dim].load(SeqCst, guard);

        // Check if the node is physically NOT within the list, or that it is there, but marked for deletion
        // If it is marked for deletion, the mdlist will physically remove it during the call to mdlist->Insert
        if !Self::is_mdnode_exist(*md_current, edge) || is_delinv(pred_child.tag()) != 0 {
            //Check if our transaction has been aborted by another thread
            let result = mdlist.insert(&new_node, md_pred, md_current, dim, pred_dim, guard);

            if result {
                *inserted = new_node.load(SeqCst, guard);
                return ReturnCode::Success;
            }
        }

        ReturnCode::Success
    }

    #[inline]
    unsafe fn insert_edge<'t>(
        &'t self,
        vertex: usize,
        edge: usize,
        value: &Option<E>,
        desc: *const Desc<'a, T, E>,
        opid: usize,
        inserted: &mut Shared<'t, MDNode<'a, E, T>>,
        md_pred: &mut Shared<'a, MDNode<'a, E, T>>,
        current: &mut Shared<'t, Node<'a, T, E>>,
        dim: &mut usize,
        pred_dim: &mut usize,
        guard: &Guard,
    ) -> ReturnCode<RefEntry<'a, 't, T, E>>
    where
        'a: 't,
    {
        let guard = &*(guard as *const _);
        *inserted = Shared::null();
        *md_pred = Shared::null();

        let n_desc = Atomic::new(NodeDesc::new(desc, opid));
        let g_n_desc = &mut n_desc.load(SeqCst, guard);

        // Try to find the vertex to which the current key is adjacenct,
        // if it is not found, we check if the vertex and edge are the same vertex.
        if self.find_vertex(current, g_n_desc, desc, vertex, guard) || vertex == edge {
            if let Some(current_ref) = current.as_ref() {
                let mdlist = &current_ref.list.as_ref().unwrap();
                let md_current = &mut mdlist.head().load(SeqCst, guard);

                let mut new_md_node = MDNode::new(edge, value.clone());
                new_md_node.node_desc = n_desc.clone();
                let new_node = Atomic::new(new_md_node);

                loop {
                    let coord = MDList::<E, T>::key_to_coord(edge);
                    MDList::locate_pred(&coord, md_pred, md_current, dim, pred_dim, guard);
                    let md_pred_ref = md_pred.as_ref().unwrap();
                    let pred_child = md_pred_ref.children[*pred_dim].load(SeqCst, guard);

                    // Check if the node is physically NOT within the list, or that it is there, but marked for deletion
                    // If it is marked for deletion, the mdlist will physically remove it during the call to mdlist->Insert
                    if !Self::is_mdnode_exist(*md_current, edge) || is_delinv(pred_child.tag()) != 0
                    {
                        // Check if our transaction has been aborted by another thread
                        match (*desc).status.load() {
                            OpStatus::Active => {}
                            _ => return ReturnCode::Fail("Transaction is inactive".into()),
                        }

                        let pred_current_desc = md_pred_ref.node_desc.load(SeqCst, guard);

                        self.finish_pending_txn(pred_current_desc.with_tag(0), desc, guard);

                        let same_op = if let (Some(a), Some(b)) =
                            (pred_current_desc.as_ref(), g_n_desc.as_ref())
                        {
                            Self::is_same_operation(a, b)
                        } else {
                            false
                        };

                        let mut pred_desc = Atomic::null();
                        if !same_op {
                            let exists =
                                if let Some(pred_current_desc_ref) = pred_current_desc.as_ref() {
                                    Self::is_key_exist(pred_current_desc_ref, guard)
                                } else {
                                    false
                                };

                            let mut new_pred_desc = NodeDesc::new(desc, opid);
                            if exists {
                                new_pred_desc.override_as_find = true;
                            } else {
                                new_pred_desc.override_as_delete = true;
                            }
                            pred_desc = Atomic::new(new_pred_desc);
                        }

                        // Update the pred node's descriptor, which provides the necessary synchronization to prevent a conflicting deleteVertex from breaking isolation
                        // There are 3 cases that InsertEdge and DeleteVertex can interleave, all cases begin after InsertEdge suceeds in updating md_pred's node_desc.
                        //
                        // Case 1: md_pred is unmarked, OR md_pred is marked but not physically removed during the InsertEdge operation
                        //      DeleteVertex will find the descriptor in md_pred and help complete the transaction before proceeding
                        // Case 2: md_pred is physically removed during the InsertEdge
                        //      DeleteVertex will find an adoption descriptor in md_pred's predecessor. This descriptor will move all children of md_pred to that node.
                        //      If InsertEdge sucessfully added it's new node to md_pred, the DeleteVertex will find it after the adoption process.
                        //      If InsertEdge is too slow to add it's new node, its CAS will fail during the insert process, and it will re-traverse
                        if same_op
                            || md_pred_ref
                                .node_desc
                                .compare_and_set(
                                    pred_current_desc,
                                    pred_desc.load(SeqCst, guard),
                                    SeqCst,
                                    guard,
                                )
                                .is_ok()
                        {
                            // Do insert
                            let result =
                                mdlist.insert(&new_node, md_pred, md_current, dim, pred_dim, guard);

                            if result {
                                *inserted = new_node.load(SeqCst, guard);
                                return ReturnCode::Success;
                            }
                        }
                    } else {
                        let current_desc =
                            md_current.as_ref().unwrap().node_desc.load(SeqCst, guard);

                        // Node needs to be deleted
                        if is_marked(current_desc.tag()) != 0 {
                            // Mark the MDList node for deletion and retry
                            // The physical deletion will occur during a call
                            // to mdlist->Insert (mdlist only performs physical deletion during insert operations)
                            let pred_child = &md_pred_ref.children[*pred_dim];
                            if is_delinv(pred_child.load(SeqCst, epoch::unprotected()).tag()) == 0
                                && pred_child
                                    .compare_and_set(
                                        *md_current,
                                        md_current.with_tag(set_delinv(md_current.tag())),
                                        SeqCst,
                                        guard,
                                    )
                                    .is_err()
                            {
                                dbg!("CAS FAILED");
                            }

                            *md_current = mdlist.head().load(SeqCst, guard);
                            *dim = 0;
                            *pred_dim = 0;
                            continue;
                        }

                        self.finish_pending_txn(current_desc, desc, guard);

                        match (current_desc.as_ref(), n_desc.load(SeqCst, guard).as_ref()) {
                            (Some(a), Some(b)) => {
                                if Self::is_same_operation(a, b) {
                                    return ReturnCode::Skip;
                                }
                            }
                            (None, _) => return ReturnCode::Fail("Current desc is NULL".into()),
                            _ => {}
                        }

                        if Self::is_key_exist(current_desc.as_ref().unwrap(), guard) {
                            return ReturnCode::Fail("Key already exists".into());
                        } else {
                            if let OpStatus::Active = (*desc).status.load() {
                            } else {
                                return ReturnCode::Fail("Transaction is Inactive".into());
                            }

                            if md_current
                                .as_ref()
                                .unwrap()
                                .node_desc
                                .compare_and_set(
                                    current_desc,
                                    n_desc.load(SeqCst, guard),
                                    SeqCst,
                                    guard,
                                )
                                .is_ok()
                            {
                                return ReturnCode::Success;
                            };
                        }
                    }
                } // End of loop
            }
        } else {
            return ReturnCode::Fail("Vertex node was not found".into());
        }

        ReturnCode::Success
    }

    unsafe fn delete_vertex<'t>(
        &'t self,
        vertex: usize,
        desc: *const Desc<'a, T, E>,
        opid: usize,
        deleted: &mut Shared<'t, Node<'a, T, E>>,
        pred: &mut Shared<'t, Node<'a, T, E>>,
        guard: &Guard,
    ) -> ReturnCode<RefEntry<'a, 't, T, E>>
    where
        'a: 't,
    {
        // Lifetime hack to bind guard to lifetime of self
        let guard = &*(guard as *const _);

        *deleted = Shared::null();
        let current = &mut self.head.load(SeqCst, guard);

        let node_desc = Atomic::new(NodeDesc::new(desc, opid));

        loop {
            self.locate_pred(pred, current, vertex, guard);

            if Self::is_node_exist(*current, vertex) {
                let current_desc = &current.as_ref().unwrap().node_desc; // Safe
                let g_current_desc = current_desc.load(SeqCst, guard);

                if is_marked(g_current_desc.tag()) != 0 {
                    return ReturnCode::Fail("Node was already marked".into());
                }

                self.finish_pending_txn(g_current_desc, desc, guard);

                if Self::is_same_operation(
                    g_current_desc.as_ref().unwrap(),
                    node_desc.load(SeqCst, guard).as_ref().unwrap(),
                ) {
                    // Check if DeleteVertex operation is ongoing
                    let pending_status = &(*desc).pending[opid];
                    if !pending_status.load() {
                        self.finish_delete_vertex(
                            current
                                .as_ref()
                                .unwrap()
                                .list
                                .as_ref()
                                .unwrap()
                                .head()
                                .load(SeqCst, guard),
                            0,
                            desc,
                            &node_desc,
                            16, // FIXME:(rasmus) Fix magic number, should be DIMENSION
                            guard,
                        );

                        // Only allow the thread that marks the operation
                        // complete to perform physical updates
                        if pending_status.compare_exchange(true, false).is_ok() {
                            *deleted = *current;
                            return ReturnCode::Deleted(RefEntry { node: *deleted });
                        }
                    }

                    return ReturnCode::Skip;
                }

                if Self::is_key_exist(g_current_desc.as_ref().unwrap(), guard) {
                    match (*desc).status.load() {
                        OpStatus::Active => {}
                        _ => return ReturnCode::Fail("Transaction is Inactive".into()),
                    }

                    if let Ok(_p) = current.as_ref().unwrap().node_desc.compare_and_set(
                        g_current_desc,
                        node_desc.load(SeqCst, guard),
                        SeqCst,
                        guard,
                    ) {
                        self.finish_delete_vertex(
                            current
                                .as_ref()
                                .unwrap()
                                .list
                                .as_ref()
                                .unwrap()
                                .head()
                                .load(SeqCst, guard),
                            0,
                            desc,
                            &node_desc,
                            16, // FIXME:(rasmus) Fix magic number, should be DIMENSION
                            guard,
                        );

                        // Only allow the thread that marks the operation
                        // complete to perform physical updates
                        let pending_status = &(*desc).pending[opid];
                        if pending_status.compare_exchange(true, false).is_ok() {
                            *deleted = *current;
                            return ReturnCode::Deleted(RefEntry { node: *deleted });
                        }
                    }
                } else {
                    return ReturnCode::Fail("Requested key does not exist".into());
                }
            } else {
                return ReturnCode::Fail("Requested node does not exist".into());
            }
        }
    }

    unsafe fn delete_edge<'t>(
        &'t self,
        vertex: usize,
        edge: usize,
        desc: *const Desc<'a, T, E>,
        opid: usize,
        deleted: &mut Shared<'t, MDNode<'a, E, T>>,
        md_pred: &mut Shared<'t, MDNode<'a, E, T>>,
        current: &mut Shared<'t, Node<'a, T, E>>,
        dim: &mut usize,
        pred_dim: &mut usize,
        guard: &Guard,
    ) -> ReturnCode<RefEntry<'a, 't, T, E>> {
        // Lifetime hack to bind guard to lifetime of self
        let guard = &*(guard as *const _);
        *deleted = Shared::null();
        *md_pred = Shared::null();
        *current = self.head.load(SeqCst, guard);

        let n_desc = Atomic::new(NodeDesc::new(desc, opid));
        let g_n_desc = &mut n_desc.load(SeqCst, guard);

        if self.find_vertex(current, g_n_desc, desc, vertex, guard) || vertex == edge {
            let md_list = &current.as_ref().unwrap().list.as_ref().unwrap();
            let md_current = &mut md_list.head().load(SeqCst, guard);
            let coord = &MDList::<T, E>::key_to_coord(edge);
            loop {
                MDList::locate_pred(coord, md_pred, md_current, dim, pred_dim, guard);

                if Self::is_mdnode_exist(*md_current, edge) {
                    let current_desc = &md_current.as_ref().unwrap().node_desc; // Safe
                    let g_current_desc = current_desc.load(SeqCst, guard);

                    if is_marked(g_current_desc.tag()) != 0 {
                        return ReturnCode::Fail("Node was already marked".into());
                    }

                    self.finish_pending_txn(g_current_desc, desc, guard);

                    if Self::is_same_operation(
                        g_current_desc.as_ref().unwrap(),
                        g_n_desc.as_ref().unwrap(),
                    ) {
                        return ReturnCode::Skip;
                    }

                    if Self::is_key_exist(g_current_desc.as_ref().unwrap(), guard) {
                        match (*desc).status.load() {
                            OpStatus::Active => {}
                            _ => return ReturnCode::Fail("Transaction is inactive".into()),
                        }
                        if md_current
                            .as_ref()
                            .unwrap()
                            .node_desc
                            .compare_and_set(g_current_desc, *g_n_desc, SeqCst, guard)
                            .is_ok()
                        {
                            *deleted = *md_current;
                            return ReturnCode::Success;
                        }
                    } else {
                        return ReturnCode::Fail("Key does not exists".into());
                    }
                } else {
                    return ReturnCode::Fail("MDNode does not exists".into());
                }
            }
        } else {
            ReturnCode::Fail("Requested vertex was not found".into())
        }
    }

    #[inline]
    unsafe fn find<'t>(
        &'t self,
        key: usize,
        desc: *const Desc<'a, T, E>,
        opid: usize,
        guard: &Guard,
    ) -> ReturnCode<RefEntry<'a, 't, T, E>>
    where
        'a: 't,
    {
        // Hack to bind lifetime of guard to self.
        let guard = &*(guard as *const _);

        let pred = &mut Shared::null();
        let current = &mut self.head.load(SeqCst, guard);

        let mut n_desc = Atomic::null();

        loop {
            self.locate_pred(pred, current, key, guard);
            if Self::is_node_exist(*current, key) {
                let current_ref = current.as_ref().unwrap();
                let current_desc = &current_ref.node_desc;

                let g_current_desc = current_desc.load(SeqCst, guard);
                if is_marked(g_current_desc.tag()) != 0 {
                    if is_marked(current_ref.next.load(SeqCst, epoch::unprotected()).tag()) == 0 {
                        current_ref.next.fetch_or(0x1, SeqCst, epoch::unprotected());
                    }
                    *current = self.head.load(SeqCst, guard);
                    continue;
                }

                self.finish_pending_txn(g_current_desc, desc, guard);

                if n_desc.load(SeqCst, guard).is_null() {
                    n_desc = Atomic::new(NodeDesc::new(desc, opid));
                }

                let current_desc_ref = g_current_desc.as_ref().unwrap();

                if Self::is_same_operation(
                    current_desc_ref,
                    n_desc.load(SeqCst, guard).as_ref().unwrap(),
                ) {
                    return ReturnCode::Skip;
                }

                if Self::is_key_exist(current_desc_ref, guard) {
                    if let OpStatus::Active = (*desc).status.load() {
                    } else {
                        return ReturnCode::Fail("Transaction is Inactive".into());
                    }

                    if current_ref
                        .node_desc
                        .compare_and_set(g_current_desc, n_desc.load(SeqCst, guard), SeqCst, guard)
                        .is_ok()
                    {
                        return ReturnCode::Found(RefEntry { node: *current });
                    }
                } else {
                    return ReturnCode::Fail("Requested key does not exist".into());
                }
            } else {
                return ReturnCode::Fail("Reqested node does not exist".into());
            }
        }
    }

    // HELPERS
    #[inline]
    unsafe fn help_ops<'t>(
        &'t self,
        desc: *const Desc<'a, T, E>,
        mut opid: usize,
        sender: &Option<std::sync::mpsc::Sender<ReturnCode<RefEntry<'a, 't, T, E>>>>,
        guard: &'g Guard,
    ) where
        'a: 't,
    {
        // FIXME:(rasmus) Safe deref_mut()?
        match (*desc).status.load() {
            OpStatus::Active => {}
            _ => return,
        }

        // Cyclic dependency check
        HELPSTACK.with(|hs| {
            for d in hs.borrow().iter() {
                if std::ptr::eq(*d as *const _, desc) {
                    (*desc)
                        .status
                        .compare_and_swap(OpStatus::Active, OpStatus::Aborted);
                    return;
                }
            }

            hs.borrow_mut().push(desc as *const _);

            let mut ret = ReturnCode::Success;

            // Vertex nodes
            let mut del_nodes: Vec<Shared<'t, Node<'a, T, E>>> = Vec::new();
            let mut del_pred_nodes: Vec<Shared<'t, Node<'a, T, E>>> = Vec::new();
            let mut ins_nodes: Vec<Shared<'t, Node<'a, T, E>>> = Vec::new();
            let mut ins_pred_nodes: Vec<Shared<'t, Node<'a, T, E>>> = Vec::new();

            // Edge nodes
            let mut md_del_nodes: Vec<Shared<'t, MDNode<'a, E, T>>> = Vec::new();
            let mut md_del_pred_nodes: Vec<Shared<'t, MDNode<'a, E, T>>> = Vec::new();
            let mut md_del_parent_nodes: Vec<Shared<'t, Node<'a, T, E>>> = Vec::new();
            let mut md_del_dims: Vec<usize> = Vec::new();
            let mut md_del_pred_dims: Vec<usize> = Vec::new();

            // Edge Nodes
            let mut md_ins_nodes: Vec<Shared<'t, MDNode<'a, E, T>>> = Vec::new();
            let mut md_ins_pred_nodes: Vec<Shared<'t, MDNode<'a, E, T>>> = Vec::new();
            let mut md_ins_parent_nodes: Vec<Shared<'t, Node<'a, T, E>>> = Vec::new();
            let mut md_ins_dims: Vec<usize> = Vec::new();
            let mut md_ins_pred_dims: Vec<usize> = Vec::new();

            while let OpStatus::Active = (*desc).status.load() {
                if let ReturnCode::Fail(_) = ret {
                    break;
                }

                if opid >= (*desc).size {
                    break;
                }
                let op = &(*desc).ops[opid];

                match &op.optype {
                    OpType::Insert(key, value) => {
                        let mut inserted = Shared::null();
                        let mut pred = Shared::null();
                        self.insert_vertex(
                            *key,
                            value,
                            desc,
                            opid,
                            &mut inserted,
                            &mut pred,
                            guard,
                        );

                        ins_nodes.push(inserted);
                        ins_pred_nodes.push(pred);
                    }

                    OpType::InsertEdge(vertex, edge, value) => {
                        let mut inserted = Shared::null();
                        let mut md_pred = Shared::null();
                        let mut parent = Shared::null();

                        let mut dim = 0;
                        let mut pred_dim = 0;

                        self.insert_edge(
                            *vertex,
                            *edge,
                            value,
                            desc,
                            opid,
                            &mut inserted,
                            &mut md_pred,
                            &mut parent,
                            &mut dim,
                            &mut pred_dim,
                            guard,
                        );

                        md_ins_nodes.push(inserted);
                        md_ins_pred_nodes.push(md_pred);
                        md_ins_parent_nodes.push(parent);
                        md_ins_dims.push(dim);
                        md_ins_pred_dims.push(pred_dim);
                    }

                    OpType::Connect(vertex, edge_id, edge) => {
                        panic!("INSTRUCTION NOT ALLOWED IN TXN");
                    }

                    OpType::Delete(vertex) => {
                        let mut deleted = Shared::null();
                        let mut pred = Shared::null();

                        self.delete_vertex(*vertex, desc, opid, &mut deleted, &mut pred, guard);

                        del_nodes.push(deleted);
                        del_pred_nodes.push(pred);
                    }

                    OpType::DeleteEdge(vertex, edge) => {
                        let mut deleted = Shared::null();
                        let mut md_pred = Shared::null();
                        let mut parent = Shared::null();

                        let mut dim = 0;
                        let mut pred_dim = 0;

                        self.delete_edge(
                            *vertex,
                            *edge,
                            desc,
                            opid,
                            &mut deleted,
                            &mut md_pred,
                            &mut parent,
                            &mut dim,
                            &mut pred_dim,
                            guard,
                        );

                        md_del_nodes.push(deleted);
                        md_del_pred_nodes.push(md_pred);
                        md_del_parent_nodes.push(parent);
                        md_del_dims.push(dim);
                        md_del_pred_dims.push(pred_dim);
                    }

                    OpType::Find(key) => {
                        ret = self.find(*key, desc, opid, guard);
                    }
                }

                opid += 1;

                sender.as_ref().map(|tx| tx.send(ret.clone()));
            }

            hs.borrow_mut().pop();

            if let ReturnCode::Fail(_) = ret {
                if (*desc)
                    .status
                    .compare_exchange(OpStatus::Active, OpStatus::Aborted)
                    .is_ok()
                {
                    // FIXME:(rasmus) call mark for deletion here
                    Self::mark_for_deletion(
                        &ins_nodes,
                        &ins_pred_nodes,
                        &md_ins_pred_nodes,
                        &md_ins_pred_nodes,
                        // &md_ins_parent_nodes,
                        &md_ins_dims,
                        &md_ins_pred_dims,
                        desc,
                        guard,
                    );
                }
            } else if (*desc)
                .status
                .compare_exchange(OpStatus::Active, OpStatus::Committed)
                .is_ok()
            {
                Self::mark_for_deletion(
                    &del_nodes,
                    &del_pred_nodes,
                    &md_del_nodes,
                    &md_del_pred_nodes,
                    // &md_del_parent_nodes,
                    &md_del_dims,
                    &md_del_pred_dims,
                    desc,
                    guard,
                )
            }
        });
    }

    #[inline]
    fn is_same_operation(desc: &NodeDesc<'a, T, E>, other: &NodeDesc<'a, T, E>) -> bool {
        std::ptr::eq(desc.desc, other.desc) && desc.opid == other.opid
    }

    #[inline]
    unsafe fn finish_pending_txn<'t>(
        &'t self,
        node_desc: Shared<NodeDesc<'a, T, E>>,
        desc: *const Desc<'a, T, E>,
        guard: &Guard,
    ) where
        'a: 't,
    {
        if let Some(node_desc_ref) = node_desc.as_ref() {
            let g_node_inner_desc = node_desc_ref.desc;

            if std::ptr::eq(g_node_inner_desc, desc) {
                return;
            }

            let optype = &(*g_node_inner_desc).ops[node_desc_ref.opid].optype;
            if let OpType::Delete(_) = optype {
                if (*g_node_inner_desc).pending[node_desc_ref.opid].load() {
                    self.help_ops(desc, node_desc_ref.opid, &None, guard);
                    return;
                }
            }

            self.help_ops(&*node_desc_ref.desc, node_desc_ref.opid, &None, guard);
        }
    }

    unsafe fn finish_delete_vertex<'t>(
        &'t self,
        n: Shared<'t, MDNode<'a, E, T>>,
        dim: usize,
        desc: *const Desc<'a, T, E>,
        node_desc: &Atomic<NodeDesc<'a, T, E>>,
        dimension: usize,
        guard: &Guard,
    ) where
        'a: 't,
    {
        // Hack to bind lifetime of guard to self.
        let guard = &*(guard as *const _);

        loop {
            let n_ref = n.as_ref().unwrap();
            let current_desc = &n_ref.node_desc;
            let g_current_desc = current_desc.load(SeqCst, guard);

            if g_current_desc.is_null() || (is_marked(g_current_desc.tag()) != 0) {
                break;
            }

            self.finish_pending_txn(g_current_desc, desc, guard);

            match (*desc).status.load() {
                OpStatus::Active => {}
                _ => return,
            }

            // Move on to the next children if we either succeed a CAS to update
            // the descriptor or we see that a different thread has already done so
            let n_node_desc = &n_ref.node_desc;
            if Self::is_same_operation(
                g_current_desc.as_ref().unwrap(),
                node_desc.load(SeqCst, guard).as_ref().unwrap(),
            ) || n_node_desc
                .compare_and_set(g_current_desc, node_desc.load(SeqCst, guard), SeqCst, guard)
                .is_ok()
            {
                let pending = n_ref.pending.load(SeqCst, guard);

                // If a pending child adoption is occuring, make sure it completes so that no nodes are missed in traversal
                // A new child adoption cannot occur at this node,
                // as our mdlist only creates adoption descriptors in new nodes during insertion
                if !pending.is_null() {
                    MDList::finish_inserting(n_ref, pending, guard);
                }

                // FIXME:(rasmus) Is this an Off-by-one-Error?
                for i in (dim..dimension).rev() {
                    let child = n_ref.children[i].load(SeqCst, guard).with_tag(0);

                    if !child.is_null() {
                        self.finish_delete_vertex(child, dim, desc, node_desc, dimension, guard);
                    }
                }

                break;
            }
        }
    }

    #[inline]
    unsafe fn is_node_exist(node: Shared<Node<'a, T, E>>, key: usize) -> bool {
        !node.is_null() && node.as_ref().unwrap().key == key
    }

    #[inline]
    unsafe fn is_mdnode_exist(node: Shared<MDNode<'a, E, T>>, key: usize) -> bool {
        !node.is_null() && node.as_ref().unwrap().key == key
    }

    #[inline]
    fn is_node_active(desc: &NodeDesc<'a, T, E>, _guard: &Guard) -> bool {
        unsafe {
            if let OpStatus::Committed = (*desc.desc).status.load() {
                true
            } else {
                false
            }
        }
    }

    /// Checks if a node is logically within the list
    #[inline]
    unsafe fn is_key_exist(node_desc: &NodeDesc<'a, T, E>, guard: &Guard) -> bool {
        let is_node_active = Self::is_node_active(node_desc, guard);
        let opoptype = &(*node_desc.desc).ops[node_desc.opid].optype;

        match opoptype {
            OpType::Find(..) => return true,
            OpType::Insert(..) | OpType::InsertEdge(..) | OpType::Connect(..) => {
                if is_node_active {
                    return true;
                }
            }
            OpType::Delete(..) | OpType::DeleteEdge(..) => {
                if !is_node_active {
                    return true;
                }
            }
        }

        node_desc.override_as_find || (!is_node_active && node_desc.override_as_delete)
    }

    #[inline]
    unsafe fn locate_pred<'t>(
        &self,
        pred: &mut Shared<'t, Node<'a, T, E>>,
        current: &mut Shared<'t, Node<'a, T, E>>,
        key: usize,
        guard: &Guard,
    ) {
        // Hack to bind lifetime of guard to self.
        let guard = &*(guard as *const _);
        let pred_next = &mut Shared::null();

        while let Some(curr_ref) = current.as_ref() {
            if curr_ref.key == key {
                break;
            }
            *pred = *current;
            let pred_n = &pred.as_ref().unwrap().next;
            *pred_next = pred_n.load(SeqCst, guard).with_tag(0);
            *current = *pred_next;

            while is_marked(curr_ref.next.load(SeqCst, epoch::unprotected()).tag()) != 0 {
                let next = curr_ref.next.load(SeqCst, guard);
                *current = next.with_tag(clr_mark(next.tag()));
            }

            if current != pred_next {
                //Failed to remove deleted nodes, start over from pred
                if pred_n
                    .compare_and_set(*pred_next, *current, SeqCst, guard)
                    .is_err()
                {
                    *current = self.head.load(SeqCst, guard);
                }
            }
        }
    }

    #[inline]
    unsafe fn find_vertex<'t>(
        &'t self,
        curr: &mut Shared<'t, Node<'a, T, E>>,
        _n_desc: &mut Shared<'t, NodeDesc<'a, T, E>>,
        desc: *const Desc<'a, T, E>,
        key: usize,
        guard: &Guard,
    ) -> bool {
        // Hack to bind lifetime of guard to self.
        let guard = &*(guard as *const _);

        *curr = self.head.load(SeqCst, guard);
        let pred = &mut Shared::null();

        self.locate_pred(pred, curr, key, guard);

        if Self::is_node_exist(*curr, key) {
            let current_desc = &curr.as_ref().unwrap().node_desc;
            let g_current_desc = current_desc.load(SeqCst, guard);
            if is_marked(g_current_desc.tag()) != 0 {
                // Node descriptor is marked for deletion
                return false;
            }

            self.finish_pending_txn(g_current_desc, desc, guard);

            Self::is_key_exist(g_current_desc.as_ref().unwrap(), guard)
        } else {
            // Vertex is not physically in the list
            false
        }
    }

    unsafe fn mark_for_deletion<'t>(
        nodes: &[Shared<'t, Node<'a, T, E>>],
        preds: &[Shared<'t, Node<'a, T, E>>],
        md_nodes: &[Shared<MDNode<'a, E, T>>],
        md_preds: &[Shared<MDNode<'a, E, T>>],
        // parents: &[Shared<'a, Node<'a, T, E>>],
        dims: &[usize],
        pred_dims: &[usize],
        desc: *const Desc<'a, T, E>,
        guard: &Guard,
    ) where
        'a: 't,
    {
        for i in 0..nodes.len() {
            let n = nodes[i];
            if !n.is_null() {
                let node_desc = &n.as_ref().unwrap().node_desc;
                let g_node_desc = node_desc.load(SeqCst, guard);

                if std::ptr::eq(g_node_desc.as_ref().unwrap().desc, desc)
                    && node_desc
                        .compare_and_set(
                            g_node_desc,
                            g_node_desc.with_tag(set_mark(g_node_desc.tag())),
                            SeqCst,
                            guard,
                        )
                        .is_ok()
                {
                    let pred = preds[i];
                    let n_next = &n.as_ref().unwrap().next;
                    let pred_next = &pred.as_ref().unwrap().next;

                    let fetched = n_next.fetch_or(0x1, SeqCst, guard);
                    let succ = fetched.with_tag(clr_mark(fetched.tag()));

                    assert!(pred_next.compare_and_set(n, succ, SeqCst, guard).is_ok());
                }
            }
        }

        for i in 0..md_nodes.len() {
            let mut node = md_nodes[i];
            let mut pred_node = md_preds[i];
            let mut dim = dims[i];
            let mut pred_dim = pred_dims[i];

            if !node.is_null() {
                let node_desc = &node.as_ref().unwrap().node_desc;
                let g_node_desc = node_desc.load(SeqCst, guard);

                if std::ptr::eq(g_node_desc.as_ref().unwrap().desc, desc)
                    && node_desc
                        .compare_and_set(
                            g_node_desc,
                            g_node_desc.with_tag(set_mark(g_node_desc.tag())),
                            SeqCst,
                            guard,
                        )
                        .is_ok()
                {
                    MDList::delete(&mut pred_node, &mut node, &mut pred_dim, &mut dim, guard);
                }
            }
        }
    }
}

impl<'a, T: Clone, E: Clone> AdjacencyList<'a, T, E> {
    pub fn txn<'t>(&'t self, mut ops: Vec<OpType<'a, T, E>>) -> Transaction<'a, 't, T, E>
    where
        'a: 't,
    {
        let ops = ops
            .drain(..)
            .map(|op| Operator::<'a, T, E> { optype: op })
            .collect();

        Transaction::new(self, ops)
    }
}

pub struct Transaction<'a: 't, 't, N, E> {
    pub status: OpStatus,
    ops: *const Desc<'a, N, E>,
    adjlist: &'t AdjacencyList<'a, N, E>,
}

impl<'a: 't, 't, N, E> Transaction<'a, 't, N, E> {
    pub fn new(adjlist: &'t AdjacencyList<'a, N, E>, operations: Vec<Operator<'a, N, E>>) -> Self {
        Transaction {
            ops: Desc::alloc(operations),
            adjlist,
            status: OpStatus::Active,
        }
    }
}

impl<'a: 't, 't, N: Clone, E: Clone> Transaction<'a, 't, N, E> {
    #[must_use]
    pub fn execute(self) -> std::sync::mpsc::Receiver<ReturnCode<RefEntry<'a, 't, N, E>>> {
        let (tx, rx) = std::sync::mpsc::channel();
        let guard = &epoch::pin();
        self.adjlist.execute_ops(self.ops, tx, guard);
        rx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type NodeData = u64;

    #[derive(Clone)]
    struct EdgeInfo<'a, T> {
        pub vertex_ref: &'a Node<'a, T, Self>,
    }

    impl<'a, T> EdgeInfo<'a, T> {
        pub fn new(vertex_ref: Shared<'a, Node<T, Self>>) -> Self {
            unsafe {
                EdgeInfo {
                    vertex_ref: vertex_ref.as_ref().unwrap(),
                }
            }
        }
    }

    #[test]
    fn insertion() {
        let adjlist = AdjacencyList::<NodeData, EdgeInfo<NodeData>>::new(5);

        let tx = adjlist.txn(vec![
            OpType::Insert(1, Some(123)),
            OpType::Insert(2, Some(123)),
            OpType::Insert(3, Some(123)),
            OpType::Insert(4, Some(123)),
            OpType::Insert(5, Some(123)),
        ]);
        let rx = tx.execute();

        let mut n_ok = 0;
        while let Ok(ReturnCode::Inserted(_)) = rx.recv() {
            n_ok += 1;
        }

        assert_eq!(n_ok, 5);
    }

    #[test]
    fn insert_and_find() {
        let adjlist = AdjacencyList::<NodeData, EdgeInfo<NodeData>>::new(5);

        let tx = adjlist.txn(vec![OpType::Insert(1, Some(123))]);
        tx.execute().recv().unwrap();

        let tx = adjlist.txn(vec![OpType::Find(1)]);
        let rx = tx.execute();

        if let Ok(ReturnCode::Found(entry)) = rx.recv() {
            assert_eq!(entry.get().value.load(), Some(123));
        }
    }

    #[test]
    fn edge_to_self() {
        let guard = &epoch::pin();
        let adjlist = AdjacencyList::<u64, u64>::new(5);

        let ops = Desc::alloc(vec![Operator {
            optype: OpType::Insert(1, Some(123)),
        }]);
        let (tx1, _rx1) = std::sync::mpsc::channel();
        adjlist.execute_ops(ops, tx1, guard);

        let ops2 = Desc::alloc(vec![Operator {
            optype: OpType::Insert(1, Some(123)),
        }]);
        let (tx2, _) = std::sync::mpsc::channel();
        adjlist.execute_ops(ops2, tx2, guard);
    }

    #[test]
    fn find_vertex() {
        let guard = &epoch::pin();
        let adjlist = AdjacencyList::<u64, u64>::new(5);
        // let empty_desc =

        let ops = Desc::alloc(vec![
            Operator {
                optype: OpType::Insert(1, Some(1)),
            },
            Operator {
                optype: OpType::Insert(2, Some(2)),
            },
            Operator {
                optype: OpType::Insert(3, Some(3)),
            },
        ]);
        let (tx1, _) = std::sync::mpsc::channel();
        adjlist.execute_ops(ops, tx1, guard);

        unsafe {
            if let ReturnCode::Found(e) = adjlist.find(1, &Desc::empty(), 0, guard) {
                assert_eq!(e.node.as_ref().unwrap().value.load(), Some(1));
            }
        }

        unsafe {
            if let ReturnCode::Found(e) = adjlist.find(2, &Desc::empty(), 0, guard) {
                assert_eq!(e.node.as_ref().unwrap().value.load(), Some(2));
            }
        }

        unsafe {
            if let ReturnCode::Found(e) = adjlist.find(3, &Desc::empty(), 0, guard) {
                assert_eq!(e.node.as_ref().unwrap().value.load(), Some(3));
            }
        }
    }

    #[test]
    fn cycle_insertion() {
        let adjlist = AdjacencyList::<NodeData, EdgeInfo<NodeData>>::new(5);
        let guard = &epoch::pin();

        let ops = Desc::alloc(vec![
            Operator {
                optype: OpType::Insert(1, Some(1)),
            },
            Operator {
                optype: OpType::Insert(2, Some(2)),
            },
        ]);
        let (tx, rx) = std::sync::mpsc::channel();
        adjlist.execute_ops(ops, tx, guard);

        if let (ReturnCode::Inserted(e1), ReturnCode::Inserted(e2)) =
            (rx.recv().unwrap(), rx.recv().unwrap())
        {
            unsafe {
                AdjacencyList::connect(&e1, 2, EdgeInfo::new(e2.node));
                AdjacencyList::connect(&e2, 1, EdgeInfo::new(e1.node));
            }
        }
    }

    #[test]
    fn find_non_existing_is_err() {
        let adjlist = AdjacencyList::<NodeData, EdgeInfo<NodeData>>::new(5);
        let find_txn = adjlist.txn(vec![OpType::Find(2)]);
        let res = find_txn.execute();
        let mut failed = false;
        if let Ok(ReturnCode::Fail(_)) = res.recv() {
            failed = true;
        }
        assert_eq!(failed, true);
    }

    #[test]
    fn large_insertion() {
        let adjlist = AdjacencyList::<NodeData, EdgeInfo<NodeData>>::new(1000);

        let mut ops = Vec::new();
        for i in 1..1000 {
            ops.push(OpType::Insert(i, Some(123)))
        }
        adjlist.txn(ops).execute().recv().unwrap();

        let mut ops = Vec::new();
        for i in 1..1000 {
            ops.push(OpType::Find(i))
        }
        let find_txn = adjlist.txn(ops);
        let res = find_txn.execute();

        let mut found_count = 0;
        while let Ok(ReturnCode::Found(_)) = res.recv() {
            found_count += 1;
        }
        assert_eq!(found_count, 1000 - 1);
    }

    // #[test]
    // fn connect() {
    //     let adjlist = AdjacencyList::<NodeData, EdgeInfo<NodeData>>::new(1, 1);

    //     let mut ops = Vec::new();
    //     ops.push(OpType::Insert(1, Some(123)));
    //     ops.push(OpType::Insert(2, Some(123)));
    //     let txn = adjlist.txn(ops).wait();

    //     if let (ReturnCode::Inserted(v1), ReturnCode::Inserted(v2)) = (&txn[0], &txn[1]) {
    //         // OK
    //         let mut ops = Vec::new();
    //         ops.push(OpType::Connect(*v1, 2, Some(EdgeInfo::new(*v2))));
    //         adjlist.txn(ops).wait();
    //     } else {
    //         panic!("Node not found");
    //     }
    // }

    // #[test]
    // fn insert_then_delete_then_find_vertex() {
    //     let adjlist = AdjacencyList::<NodeData, EdgeInfo<NodeData>>::new(1, 1);

    //     let mut ops = Vec::new();
    //     ops.push(OpType::Insert(1, Some(123)));
    //     adjlist.txn(ops).wait();

    //     let mut ops = Vec::new();
    //     ops.push(OpType::Delete(1));
    //     adjlist.txn(ops).wait();

    //     let mut ops = Vec::new();
    //     ops.push(OpType::Find(1));
    //     let res = adjlist.txn(ops).wait();

    //     match res[0] {
    //         ReturnCode::Fail => {}
    //         _ => panic!("Node was found"),
    //     }
    // }

    // #[test]
    // fn held_reference_is_valid_after_delete() {
    //     let adjlist = AdjacencyList::<NodeData, EdgeInfo<NodeData>>::new(1, 1);

    //     let mut ops = Vec::new();
    //     ops.push(OpType::Insert(1, Some(123)));
    //     let ret = adjlist.txn(ops).wait();

    //     let reference = match ret[0] {
    //         ReturnCode::Inserted(e) => Some(*e),
    //         _ => None,
    //     }
    //     .and_then(|r| unsafe { r.as_ref() })
    //     .unwrap();

    //     let mut ops = Vec::new();
    //     ops.push(OpType::Delete(1));
    //     adjlist.txn(ops).wait();

    //     assert_eq!(reference.value(), Some(123));
    // }

    // #[test]
    // fn held_reference_is_not_changed_after_reinsertion() {
    //     let adjlist = AdjacencyList::<NodeData, EdgeInfo<NodeData>>::new(1, 1);

    //     let mut ops = Vec::new();
    //     ops.push(OpType::Insert(1, Some(123)));
    //     let ret = adjlist.txn(ops).wait();

    //     let reference = match ret[0] {
    //         ReturnCode::Inserted(e) => Some(*e),
    //         _ => None,
    //     }
    //     .and_then(|r| unsafe { r.as_ref() })
    //     .unwrap();

    //     let mut ops = Vec::new();
    //     ops.push(OpType::Delete(1));
    //     adjlist.txn(ops).wait();

    //     let mut ops = Vec::new();
    //     ops.push(OpType::Insert(1, Some(345)));
    //     adjlist.txn(ops).wait();

    //     assert_eq!(reference.value(), Some(123));
    // }

    // #[test]
    // fn held_reference_is_changed_after_update() {
    //     let adjlist = AdjacencyList::<NodeData, EdgeInfo<NodeData>>::new(1, 1);

    //     let mut ops = Vec::new();
    //     ops.push(OpType::Insert(1, Some(123)));
    //     let ret = adjlist.txn(ops).wait();

    //     let reference = match ret[0] {
    //         ReturnCode::Inserted(e) => Some(*e),
    //         _ => None,
    //     }
    //     .and_then(|r| unsafe { r.as_ref() })
    //     .unwrap();

    //     let mut ops = Vec::new();
    //     ops.push(OpType::Insert(1, Some(345)));
    //     adjlist.txn(ops).wait();

    //     assert_eq!(reference.value(), Some(345));
    // }

    // #[test]
    // fn delete_edge_created_with_insert_edge() {
    //     let adjlist = AdjacencyList::<NodeData, EdgeInfo<NodeData>>::new(1, 1);
    //     let guard = &epoch::pin();
    //     // Insert Nodes
    //     let mut ops = Vec::new();
    //     ops.push(OpType::Insert(1, Some(123)));
    //     ops.push(OpType::Insert(2, Some(123)));
    //     let insertions = adjlist.txn(ops).wait();

    //     // Insert Edge
    //     if let (ReturnCode::Inserted(v1), ReturnCode::Inserted(v2)) =
    //         (&insertions[0], &insertions[1])
    //     {
    //         // OK
    //         let mut ops = Vec::new();
    //         ops.push(OpType::InsertEdge(1, 2, Some(EdgeInfo::new(*v2))));
    //         adjlist.txn(ops).wait();
    //     } else {
    //         panic!("Node not found");
    //     }

    //     // Delete edge
    //     let mut ops = Vec::new();
    //     ops.push(OpType::DeleteEdge(1, 2));
    //     let deletions = adjlist.txn(ops).wait();
    //     if let ReturnCode::Success = &deletions[0] {
    //     } else {
    //         panic!("Could not delete edge");
    //     }

    //     let mut ops = Vec::new();
    //     ops.push(OpType::Find(1));
    //     let res = adjlist.txn(ops).wait();

    //     if let ReturnCode::Found(v) = &res[0] {
    //         unsafe {
    //             let vertex_ref = v.as_ref().unwrap();
    //             assert_eq!(
    //                 vertex_ref.list.as_ref().unwrap().get(2, guard).is_err(),
    //                 true
    //             )
    //         }
    //     } else {
    //         panic!("Node not found");
    //     }
    // }

    // #[test]
    // fn deleted_edge_is_not_found() {
    //     let adjlist = AdjacencyList::<NodeData, EdgeInfo<NodeData>>::new(1, 1);
    //     let guard = &epoch::pin();
    //     // Insert Nodes
    //     let mut ops = Vec::new();
    //     ops.push(OpType::Insert(1, Some(123)));
    //     ops.push(OpType::Insert(2, Some(123)));
    //     let insertions = adjlist.txn(ops).wait();

    //     // Insert Edge
    //     if let (ReturnCode::Inserted(v1), ReturnCode::Inserted(v2)) =
    //         (&insertions[0], &insertions[1])
    //     {
    //         // OK
    //         let mut ops = Vec::new();
    //         ops.push(OpType::InsertEdge(1, 2, Some(EdgeInfo::new(*v2))));
    //         adjlist.txn(ops).wait();
    //     } else {
    //         panic!("Node not found");
    //     }

    //     // Delete edge
    //     let mut ops = Vec::new();
    //     ops.push(OpType::DeleteEdge(1, 2));
    //     adjlist.txn(ops).wait();

    //     let mut ops = Vec::new();
    //     ops.push(OpType::Find(1));
    //     let res = adjlist.txn(ops).wait();

    //     if let ReturnCode::Found(v) = &res[0] {
    //         unsafe {
    //             let vertex_ref = v.as_ref().unwrap();
    //             let child = vertex_ref.list.as_ref().unwrap().get(2, guard);
    //             assert_eq!(child.is_err(), true);
    //         }
    //     } else {
    //         panic!("Node not found");
    //     }
    // }

    // #[test]
    // fn get_child_info() {
    //     let adjlist = AdjacencyList::<NodeData, EdgeInfo<NodeData>>::new(1, 1);
    //     let guard = &epoch::pin();
    //     // Insert Nodes
    //     let mut ops = Vec::new();
    //     ops.push(OpType::Insert(1, Some(123)));
    //     ops.push(OpType::Insert(2, Some(456)));
    //     let insertions = adjlist.txn(ops).wait();

    //     // Insert Edge
    //     if let (ReturnCode::Inserted(v1), ReturnCode::Inserted(v2)) =
    //         (&insertions[0], &insertions[1])
    //     {
    //         let mut ops = Vec::new();
    //         ops.push(OpType::Connect(*v1, 2, Some(EdgeInfo::new(*v2))));
    //         adjlist.txn(ops).wait();
    //     } else {
    //         panic!("Node not found");
    //     }

    //     let mut ops = Vec::new();
    //     ops.push(OpType::Find(1));
    //     let res = adjlist.txn(ops).wait();

    //     if let ReturnCode::Found(v) = &res[0] {
    //         unsafe {
    //             let vertex_ref = v.as_ref().unwrap();
    //             let child = vertex_ref.list.as_ref().unwrap().get(2, guard).unwrap();
    //             assert_eq!(
    //                 child.value().unwrap().vertex_ref.as_ref().unwrap().value(),
    //                 Some(456)
    //             )
    //         }
    //     } else {
    //         panic!("Node not found");
    //     }
    // }

    // #[test]
    // fn get_all_children() {
    //     let adjlist = AdjacencyList::<NodeData, EdgeInfo<NodeData>>::new(1, 1);
    //     let guard = &epoch::pin();
    //     // Insert Nodes
    //     let mut ops = Vec::new();
    //     ops.push(OpType::Insert(1, Some(123)));
    //     ops.push(OpType::Insert(2, Some(456)));
    //     ops.push(OpType::Insert(3, Some(789)));
    //     let insertions = adjlist.txn(ops).wait();

    //     // Insert Edge
    //     if let (ReturnCode::Inserted(v1), ReturnCode::Inserted(v2), ReturnCode::Inserted(v3)) =
    //         (&insertions[0], &insertions[1], &insertions[2])
    //     {
    //         let mut ops = Vec::new();
    //         ops.push(OpType::Connect(*v1, 2, Some(EdgeInfo::new(*v2))));
    //         ops.push(OpType::Connect(*v1, 3, Some(EdgeInfo::new(*v3))));
    //         adjlist.txn(ops).wait();
    //     } else {
    //         panic!("Node not found");
    //     }

    //     let mut ops = Vec::new();
    //     ops.push(OpType::Find(1));
    //     let res = adjlist.txn(ops).wait();

    //     if let ReturnCode::Found(v) = &res[0] {
    //         unsafe {
    //             let vertex_ref = v.as_ref().unwrap();
    //             let children = vertex_ref.list.as_ref().unwrap().entries(guard);

    //             assert_eq!(children.len(), 2);
    //         }
    //     } else {
    //         panic!("Node not found");
    //     }
    // }

    // #[test]
    // fn delete_child_updates_node_links() {
    //     let adjlist = AdjacencyList::<NodeData, EdgeInfo<NodeData>>::new(1, 1);
    //     let guard = &epoch::pin();
    //     // Insert Nodes
    //     let mut ops = Vec::new();
    //     ops.push(OpType::Insert(1, Some(1)));
    //     ops.push(OpType::Insert(2, Some(2)));
    //     ops.push(OpType::Insert(3, Some(3)));
    //     ops.push(OpType::Insert(4, Some(4)));
    //     let insertions = adjlist.txn(ops).wait();

    //     // Insert Edge
    //     if let (
    //         ReturnCode::Inserted(v1),
    //         ReturnCode::Inserted(v2),
    //         ReturnCode::Inserted(v3),
    //         ReturnCode::Inserted(v4),
    //     ) = (
    //         &insertions[0],
    //         &insertions[1],
    //         &insertions[2],
    //         &insertions[3],
    //     ) {
    //         let mut ops = Vec::new();
    //         ops.push(OpType::Connect(*v1, 2, Some(EdgeInfo::new(*v2))));
    //         ops.push(OpType::Connect(*v1, 3, Some(EdgeInfo::new(*v3))));
    //         ops.push(OpType::Connect(*v1, 4, Some(EdgeInfo::new(*v4))));

    //         adjlist.txn(ops).wait();
    //     } else {
    //         panic!("Node not found");
    //     }

    //     let mut ops = Vec::new();
    //     ops.push(OpType::Find(1));
    //     ops.push(OpType::DeleteEdge(1, 3));
    //     let res = adjlist.txn(ops).wait();

    //     // Check that 2 and 4 is still found...
    //     if let ReturnCode::Found(v) = &res[0] {
    //         unsafe {
    //             let vertex_ref = v.as_ref().unwrap();
    //             let children = vertex_ref.list.as_ref().unwrap().entries(guard);

    //             assert_eq!(children.len(), 2);
    //         }
    //     } else {
    //         panic!("Node not found");
    //     }
    // }

    //     #[test]
    //     fn insert_edge_at_vertex() {
    //         let adjlist = AdjacencyList::new(1, 1);

    //         let desc = Atomic::new(Desc::new(vec![
    //             Operator {
    //                 optype: OpType::Insert(1, Some("abc")),
    //                 retval: None,
    //             },
    //             Operator {
    //                 optype: OpType::Insert(2, Some("abc")),
    //                 retval: None,
    //             },
    //         ]));
    //         assert_eq!(adjlist.execute_ops(&desc), true);

    //         let desc = Atomic::new(Desc::new(vec![Operator {
    //             optype: OpType::InsertEdge(1, 2),
    //             retval: None,
    //         }]));
    //         assert_eq!(adjlist.execute_ops(&desc), true);
    //     }

    //     #[test]
    //     fn insert_edge_at_non_existing_vertex_is_err() {
    //         let adjlist = AdjacencyList::<u64>::new(1, 1);

    //         let desc = Atomic::new(Desc::new(vec![Operator {
    //             optype: OpType::InsertEdge(1, 2),
    //             retval: None,
    //         }]));
    //         assert_eq!(adjlist.execute_ops(&desc), false);
    //     }

    //     #[test]
    //     fn insert_existing_vertex_as_edge() {
    //         let adjlist = AdjacencyList::new(1, 1);

    //         let desc = Atomic::new(Desc::new(vec![
    //             Operator {
    //                 optype: OpType::Insert(1, Some("abc")),
    //                 retval: None,
    //             },
    //             Operator {
    //                 optype: OpType::Insert(2, Some("abc")),
    //                 retval: None,
    //             },
    //         ]));
    //         assert_eq!(adjlist.execute_ops(&desc), true);

    //         let desc = Atomic::new(Desc::new(vec![Operator {
    //             optype: OpType::InsertEdge(1, 2),
    //             retval: None,
    //         }]));
    //         assert_eq!(adjlist.execute_ops(&desc), true);
    //     }

    //     #[test]
    //     fn insert_same_edge_twice() {
    //         let adjlist = AdjacencyList::new(1, 1);

    //         let desc = Atomic::new(Desc::new(vec![
    //             Operator {
    //                 optype: OpType::Insert(1, Some("abc")),
    //                 retval: None,
    //             },
    //             Operator {
    //                 optype: OpType::Insert(2, Some("abc")),
    //                 retval: None,
    //             },
    //         ]));
    //         assert_eq!(adjlist.execute_ops(&desc), true);

    //         let desc = Atomic::new(Desc::new(vec![Operator {
    //             optype: OpType::InsertEdge(1, 2),
    //             retval: None,
    //         }]));
    //         assert_eq!(adjlist.execute_ops(&desc), true);

    //         let desc = Atomic::new(Desc::new(vec![Operator {
    //             optype: OpType::InsertEdge(1, 2),
    //             retval: None,
    //         }]));
    //         assert_eq!(adjlist.execute_ops(&desc), false);
    //     }

    //     #[test]
    //     fn referenced_edge_is_updated_on_vertex_update() {
    //         let guard = &epoch::pin();
    //         let adjlist = AdjacencyList::new(1, 1);

    //         let desc = Atomic::new(Desc::new(vec![
    //             Operator {
    //                 optype: OpType::Insert(1, Some("Root Node")),
    //                 retval: None,
    //             },
    //             Operator {
    //                 optype: OpType::Insert(2, Some("Edge Node")),
    //                 retval: None,
    //             },
    //         ]));
    //         assert_eq!(adjlist.execute_ops(&desc), true);

    //         let desc = Atomic::new(Desc::new(vec![Operator {
    //             optype: OpType::InsertEdge(1, 2),
    //             retval: None,
    //         }]));
    //         assert_eq!(adjlist.execute_ops(&desc), true);

    //         let desc = Atomic::new(Desc::new(vec![Operator {
    //             optype: OpType::Insert(2, Some("Updated Edge Node")),
    //             retval: None,
    //         }]));
    //         assert_eq!(adjlist.execute_ops(&desc), true);

    //         let find = Atomic::new(Desc::new(vec![Operator {
    //             optype: OpType::Find(1),
    //             retval: None,
    //         }]));
    //         assert_eq!(adjlist.execute_ops(&find), true);

    //         let mut reached = false;
    //         unsafe {
    //             for op in &find.load(SeqCst, guard).deref().ops {
    //                 if let Some(ReturnCode::Found(ref vertex)) = op.retval {
    //                     reached = true;
    //                     assert_eq!(
    //                         vertex
    //                             .deref()
    //                             .list
    //                             .as_ref()
    //                             .unwrap()
    //                             .get(2, guard)
    //                             .unwrap()
    //                             .value(),
    //                         Some("Updated Edge Node")
    //                     );
    //                 }
    //             }
    //         }
    //         assert!(reached);
    //     }
}
