use std::slice::Iter;
use std::slice::IterMut;

pub struct SlidingQueue<T> {
    inner: Vec<T>,
    shared_in: usize,
    shared_out_start: usize,
    shared_out_end: usize,
}

impl<T> SlidingQueue<T> {
    pub fn new() -> Self {
        Self {
            inner: Vec::new(),
            shared_in: 0,
            shared_out_start: 0,
            shared_out_end: 0,
        }
    }

    pub fn with_capacity(size_hint: usize) -> Self {
        Self {
            inner: Vec::with_capacity(size_hint),
            shared_in: 0,
            shared_out_start: 0,
            shared_out_end: 0,
        }
    }

    pub fn push_back(&mut self, entry: T) {
        self.shared_in += 1;
        self.inner.push(entry);
    }

    pub fn size(&self) -> usize {
        self.shared_out_end
    }

    pub fn empty(&self) -> bool {
        self.shared_out_start == self.shared_out_end
    }

    pub fn reset(&mut self) {
        self.shared_out_start = 0;
        self.shared_out_end = 0;
        self.shared_in = 0;
    }

    pub fn slide_window(&mut self) {
        self.shared_out_start = self.shared_out_end;
        self.shared_out_end = self.shared_in;
    }
}

impl<T> IntoIterator for SlidingQueue<T> {
    type Item = T;
    type IntoIter = std::iter::Take<std::iter::Skip<std::vec::IntoIter<Self::Item>>>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner
            .into_iter()
            .skip(self.shared_out_start)
            .take(self.shared_out_end)
    }
}

impl<'a, T> IntoIterator for &'a SlidingQueue<T> {
    type Item = &'a T;
    type IntoIter = std::iter::Take<std::iter::Skip<Iter<'a, T>>>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner
            .iter()
            .skip(self.shared_out_start)
            .take(self.shared_out_end)
    }
}

impl<'a, T> IntoIterator for &'a mut SlidingQueue<T> {
    type Item = &'a mut T;
    type IntoIter = std::iter::Take<std::iter::Skip<IterMut<'a, T>>>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner
            .iter_mut()
            .skip(self.shared_out_start)
            .take(self.shared_out_end)
    }
}
