//! Slice sorting: a stable natural merge sort (TimSort-like run detection
//! and merging). The unstable sorts and selection are std's ipnsort /
//! introselect in `unstable_sort.rs`.
//!
//! As in the real library, small slices (at most 20 elements) are sorted
//! with insertion sort by every method (so even the unstable sorts keep the
//! order of equal elements there), and a comparison function that panics
//! leaves the slice holding every one of its original elements.

use crate::intrinsics;
use crate::mem::{self, MaybeUninit};

const MAX_INSERTION: usize = 20;

#[inline(always)]
unsafe fn at<T>(v: *mut T, i: usize) -> *mut T {
    intrinsics::ptr_add_mut(v, i)
}

#[inline(always)]
unsafe fn swap<T>(v: *mut T, i: usize, j: usize) {
    crate::ptr::swap(at(v, i), at(v, j))
}

/// Copies `len` elements from `src` to `dst` when dropped (closes the gap a
/// moved-out element left, also when a comparison panics).
struct CopyOnDrop<T> {
    src: *const T,
    dst: *mut T,
    len: usize,
}

impl<T> Drop for CopyOnDrop<T> {
    #[inline]
    fn drop(&mut self) {
        unsafe { intrinsics::copy_nonoverlapping(self.src, self.dst, self.len) }
    }
}

/// Sorts `v[..len]` with insertion sort, given that `v[..offset]` is
/// already sorted (`offset >= 1`). Stable.
unsafe fn insertion_sort<T, F: FnMut(&T, &T) -> bool>(v: *mut T, len: usize, offset: usize, is_less: &mut F) {
    let mut tmp = MaybeUninit::<T>::uninit();
    let tp = tmp.as_mut_ptr();
    // while an element is out (in `tmp`), the guard's `len` is 1
    let mut gap = CopyOnDrop { src: tp as *const T, dst: v, len: 0 };
    let mut i = offset;
    while i < len {
        let p_i = at(v, i);
        if is_less(&*p_i, &*at(v, i - 1)) {
            intrinsics::copy_nonoverlapping(p_i as *const T, tp, 1);
            intrinsics::copy_nonoverlapping(at(v, i - 1) as *const T, p_i, 1);
            let mut j = i - 1;
            gap.dst = at(v, j);
            gap.len = 1;
            while j > 0 && is_less(&*tp, &*at(v, j - 1)) {
                intrinsics::copy_nonoverlapping(at(v, j - 1) as *const T, at(v, j), 1);
                j -= 1;
                gap.dst = at(v, j);
            }
            intrinsics::copy_nonoverlapping(tp as *const T, at(v, j), 1);
            gap.len = 0;
        }
        i += 1;
    }
}

/// Returns the length of the run at the start of `v[..len]`, made
/// non-descending (a strictly descending run is reversed, which keeps the
/// sort stable).
unsafe fn find_run<T, F: FnMut(&T, &T) -> bool>(v: *mut T, len: usize, is_less: &mut F) -> usize {
    if len < 2 {
        return len;
    }
    let mut end = 2;
    if is_less(&*at(v, 1), &*at(v, 0)) {
        while end < len && is_less(&*at(v, end), &*at(v, end - 1)) {
            end += 1;
        }
        let mut i = 0;
        let mut j = end - 1;
        while i < j {
            swap(v, i, j);
            i += 1;
            j -= 1;
        }
    } else {
        while end < len && !is_less(&*at(v, end), &*at(v, end - 1)) {
            end += 1;
        }
    }
    end
}

/// While merging, the elements `buf[start..end]` still belong at `dst..`
/// (copied back if a comparison panics).
struct MergeHole<T> {
    start: *mut T,
    end: *mut T,
    dst: *mut T,
}

impl<T> Drop for MergeHole<T> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            let n = intrinsics::ptr_diff(self.end as *const T, self.start as *const T) as usize;
            intrinsics::copy_nonoverlapping(self.start as *const T, self.dst, n);
        }
    }
}

/// Merges the sorted runs `v[..mid]` and `v[mid..len]` (stably), with a
/// buffer `buf` that holds the shorter of the two.
unsafe fn merge<T, F: FnMut(&T, &T) -> bool>(v: *mut T, len: usize, mid: usize, buf: *mut T, is_less: &mut F) {
    let rlen = len - mid;
    if mid <= rlen {
        // the left run goes to the buffer; merge front to back
        intrinsics::copy_nonoverlapping(v as *const T, buf, mid);
        let mut hole = MergeHole { start: buf, end: at(buf, mid), dst: v };
        let mut l = buf;
        let lend = at(buf, mid);
        let mut r = at(v, mid);
        let rend = at(v, len);
        let mut out = v;
        while l < lend && r < rend {
            if is_less(&*r, &*l) {
                intrinsics::copy_nonoverlapping(r as *const T, out, 1);
                r = at(r, 1);
            } else {
                intrinsics::copy_nonoverlapping(l as *const T, out, 1);
                l = at(l, 1);
                hole.start = l;
            }
            out = at(out, 1);
            hole.dst = out;
        }
        // (the hole copies what is left of the buffer)
        drop(hole);
    } else {
        // the right run goes to the buffer; merge back to front
        intrinsics::copy_nonoverlapping(at(v, mid) as *const T, buf, rlen);
        let mut hole = MergeHole { start: buf, end: at(buf, rlen), dst: at(v, mid) };
        let mut l = at(v, mid); // one past the left element to compare
        let mut r = at(buf, rlen); // one past the right element to compare
        let mut out = at(v, len);
        while v < l && buf < r {
            let lp = intrinsics::ptr_offset_mut(l, -1);
            let rp = intrinsics::ptr_offset_mut(r, -1);
            out = intrinsics::ptr_offset_mut(out, -1);
            if is_less(&*rp, &*lp) {
                intrinsics::copy_nonoverlapping(lp as *const T, out, 1);
                l = lp;
            } else {
                intrinsics::copy_nonoverlapping(rp as *const T, out, 1);
                r = rp;
                hole.end = r;
            }
            hole.dst = l;
        }
        // what is left of the buffer goes right after the remaining left run
        drop(hole);
    }
}

/// Frees the merge buffer.
struct Buffer<T> {
    ptr: *mut T,
    len: usize,
}

impl<T> Drop for Buffer<T> {
    fn drop(&mut self) {
        unsafe { crate::alloc::dealloc_array(self.ptr, self.len) }
    }
}

/// The run length below which runs are extended with insertion sort (as in
/// TimSort: between 16 and 32, such that len / min_run is close to a power of 2).
fn min_run_len(mut n: usize) -> usize {
    let mut r = 0;
    while n >= 32 {
        r |= n & 1;
        n >>= 1;
    }
    n + r
}

/// The pending runs of the merge sort (TimSort's run stack): (start, len).
struct Runs {
    runs: [(usize, usize); 80],
    n: usize,
}

impl Runs {
    /// The runs to merge next, `r` and `r + 1` (TimSort's rules, which
    /// keep the stack short), if any; `at_end`: no run is left to add.
    fn to_merge(&self, at_end: bool) -> Option<usize> {
        let n = self.n;
        let l = |i: usize| self.runs[i].1;
        if n >= 2
            && (at_end
                || l(n - 2) <= l(n - 1)
                || (n >= 3 && l(n - 3) <= l(n - 2) + l(n - 1))
                || (n >= 4 && l(n - 4) <= l(n - 3) + l(n - 2)))
        {
            Some(if n >= 3 && l(n - 3) < l(n - 1) { n - 3 } else { n - 2 })
        } else {
            None
        }
    }

    /// Records that runs `r` and `r + 1` were merged.
    fn merged(&mut self, r: usize) {
        self.runs[r].1 += self.runs[r + 1].1;
        let mut k = r + 1;
        while k + 1 < self.n {
            self.runs[k] = self.runs[k + 1];
            k += 1;
        }
        self.n -= 1;
    }
}

/// Sorts the slice stably with the strict-weak-order predicate `is_less`.
pub(crate) fn merge_sort<T, F: FnMut(&T, &T) -> bool>(v: &mut [T], is_less: &mut F) {
    let len = v.len();
    if len < 2 || mem::size_of::<T>() == 0 {
        return;
    }
    let p = v.as_mut_ptr();
    unsafe {
        if len <= MAX_INSERTION {
            insertion_sort(p, len, 1, is_less);
            return;
        }
        // fully sorted or strictly descending input: no buffer needed
        let first = find_run(p, len, is_less);
        if first == len {
            return;
        }
        let buf = Buffer { ptr: crate::alloc::alloc_array::<T>(len / 2), len: len / 2 };
        let min_run = min_run_len(len);
        let mut runs = Runs { runs: [(0, 0); 80], n: 0 };
        let mut start = 0;
        while start < len {
            let mut rl = if start == 0 { first } else { find_run(at(p, start), len - start, is_less) };
            if rl < min_run {
                let end = if start + min_run < len { start + min_run } else { len };
                insertion_sort(at(p, start), end - start, rl, is_less);
                rl = end - start;
            }
            runs.runs[runs.n] = (start, rl);
            runs.n += 1;
            start += rl;
            while let Some(r) = runs.to_merge(start == len) {
                let (ls, ll) = runs.runs[r];
                let rl2 = runs.runs[r + 1].1;
                // (skipped when the runs are already in order)
                if is_less(&*at(p, ls + ll), &*at(p, ls + ll - 1)) {
                    merge(at(p, ls), ll + rl2, ll, buf.ptr, is_less);
                }
                runs.merged(r);
            }
        }
        drop(buf);
    }
}
