use crate::graphmodels::epoch::adjlist::RefEntry;
use crossbeam_utils::atomic::AtomicCell;

use std::alloc::{alloc, dealloc, handle_alloc_error, Layout};
use std::mem;
use std::ptr;

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum OpStatus {
    Active,
    Committed,
    Aborted,
}

#[derive(Debug, Clone)]
pub enum ReturnCode<R> {
    Success,
    Inserted(R),
    Deleted(R),
    Found(R),
    Skip,
    Fail(String),
}

#[derive(Clone)]
pub enum OpType<'a, T, E> {
    Find(usize),
    Insert(usize, Option<T>),
    Connect(&'a RefEntry<'a, 'a, T, E>, usize, E),
    Delete(usize),
    InsertEdge(usize, usize, Option<E>),
    DeleteEdge(usize, usize),
}

pub struct Operator<'a, T, E> {
    pub optype: OpType<'a, T, E>,
}

pub struct Desc<'a, T, E> {
    pub status: AtomicCell<OpStatus>,
    pub size: usize,
    pub ops: Vec<Operator<'a, T, E>>,
    pub pending: Vec<AtomicCell<bool>>,
}

impl<'a, T: 'a, E: 'a> Desc<'a, T, E> {
    // pub fn new(ops: Vec<Operator<T, E>>) -> Self {
    //     let size = ops.len();
    //     Desc {
    //         status: AtomicCell::new(OpStatus::Active),
    //         size,
    //         ops,
    //         pending: (0..size).map(|_| AtomicCell::new(true)).collect(),
    //     }
    // }
    #[must_use]
    pub fn alloc(ops: Vec<Operator<'a, T, E>>) -> *mut Self {
        unsafe {
            let layout = Self::get_layout();
            #[allow(clippy::cast_ptr_alignment)]
            let ptr = alloc(layout) as *mut Self;
            if ptr.is_null() {
                handle_alloc_error(layout);
            }

            ptr::write(&mut (*ptr).status, AtomicCell::new(OpStatus::Active));

            let size = ops.len();
            ptr::write(&mut (*ptr).size, size);

            ptr::write(&mut (*ptr).ops, ops);

            ptr::write(
                &mut (*ptr).pending,
                (0..size).map(|_| AtomicCell::new(true)).collect(),
            );

            ptr
        }
    }

    /// Deallocates a node.
    ///
    /// This function will not run any destructors.
    ///
    /// # Safety
    ///
    /// Be careful not to deallocate data that is still in use.
    pub unsafe fn dealloc(ptr: *mut Self) {
        let layout = Self::get_layout();
        dealloc(ptr as *mut u8, layout);
    }

    /// Returns the layout of a node with the given `height`.
    unsafe fn get_layout() -> Layout {
        let size_self = mem::size_of::<Self>();
        let align_self = mem::align_of::<Self>();

        Layout::from_size_align_unchecked(size_self, align_self)
    }

    #[must_use]
    pub fn empty() -> Self {
        Self {
            status: AtomicCell::new(OpStatus::Committed),
            size: 0,
            ops: Vec::new(),
            pending: Vec::new(),
        }
    }
}

pub struct NodeDesc<'a, T, E> {
    pub desc: *const Desc<'a, T, E>,
    pub opid: usize,
    pub override_as_find: bool,
    pub override_as_delete: bool,
}

impl<'a, T, E> Drop for NodeDesc<'a, T, E> {
    fn drop(&mut self) {
        unsafe { Desc::dealloc(self.desc as *mut Desc<T, E>) }
    }
}

impl<'a, T, E> NodeDesc<'a, T, E> {
    #[inline]
    pub fn new(desc: *const Desc<'a, T, E>, opid: usize) -> Self {
        Self {
            desc,
            opid,
            override_as_find: false,
            override_as_delete: false,
        }
    }
}
