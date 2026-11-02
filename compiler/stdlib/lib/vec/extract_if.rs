//! `Vec::extract_if`.

use crate::fmt;
use crate::ptr;

use super::Vec;

/// An iterator which uses a closure to determine if an element should be removed.
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct ExtractIf<'a, T, F> {
    vec: &'a mut Vec<T>,
    /// The index of the item that will be inspected by the next call to `next`.
    idx: usize,
    /// Elements at and beyond this point will be retained.
    end: usize,
    /// The number of items that have been drained (removed) thus far.
    del: usize,
    /// The original length of `vec` prior to draining.
    old_len: usize,
    pred: F,
}

impl<'a, T, F> ExtractIf<'a, T, F> {
    pub(super) fn new(vec: &'a mut Vec<T>, pred: F, start: usize, end: usize) -> Self {
        let old_len = vec.len();
        // Guard against the vector getting leaked (leak amplification).
        unsafe {
            vec.set_len(0);
        }
        ExtractIf { vec, idx: start, del: 0, end, old_len, pred }
    }
}

impl<T, F> Iterator for ExtractIf<'_, T, F>
where
    F: FnMut(&mut T) -> bool,
{
    type Item = T;

    fn next(&mut self) -> Option<T> {
        unsafe {
            while self.idx < self.end {
                let i = self.idx;
                let p = self.vec.as_mut_ptr();
                let drained = (self.pred)(&mut *p.add(i));
                // Update the index *after* the predicate is called. If the index
                // is updated prior and the predicate panics, the element at this
                // index would be leaked.
                self.idx += 1;
                if drained {
                    self.del += 1;
                    return Some(ptr::read(p.add(i)));
                } else if self.del > 0 {
                    let del = self.del;
                    ptr::copy_nonoverlapping(p.add(i), p.add(i - del), 1);
                }
            }
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.end - self.idx))
    }
}

impl<T, F> Drop for ExtractIf<'_, T, F> {
    fn drop(&mut self) {
        unsafe {
            if self.idx < self.old_len && self.del > 0 {
                let p = self.vec.as_mut_ptr();
                let src = p.add(self.idx);
                let dst = src.sub(self.del);
                let tail_len = self.old_len - self.idx;
                ptr::copy(src, dst, tail_len);
            }
            self.vec.set_len(self.old_len - self.del);
        }
    }
}

impl<T: fmt::Debug, F> fmt::Debug for ExtractIf<'_, T, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExtractIf").finish_non_exhaustive()
    }
}
