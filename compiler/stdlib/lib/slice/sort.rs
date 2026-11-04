//! Slice sorting: a stable natural merge sort (TimSort-like run detection
//! and merging), an unstable pattern-defeating quicksort and quickselect.
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
        // the pending runs (start, len), merged TimSort-style
        let mut runs: [(usize, usize); 80] = [(0, 0); 80];
        let mut n = 0;
        let mut start = 0;
        while start < len {
            let mut rl = if start == 0 { first } else { find_run(at(p, start), len - start, is_less) };
            if rl < min_run {
                let end = if start + min_run < len { start + min_run } else { len };
                insertion_sort(at(p, start), end - start, rl, is_less);
                rl = end - start;
            }
            runs[n] = (start, rl);
            n += 1;
            start += rl;
            loop {
                let ok = n >= 2
                    && (start == len
                        || runs[n - 2].1 <= runs[n - 1].1
                        || (n >= 3 && runs[n - 3].1 <= runs[n - 2].1 + runs[n - 1].1)
                        || (n >= 4 && runs[n - 4].1 <= runs[n - 3].1 + runs[n - 2].1));
                if !ok {
                    break;
                }
                let r = if n >= 3 && runs[n - 3].1 < runs[n - 1].1 { n - 3 } else { n - 2 };
                let (ls, ll) = runs[r];
                let (_, rl2) = runs[r + 1];
                // skip the merge when the runs are already in order
                if is_less(&*at(p, ls + ll), &*at(p, ls + ll - 1)) {
                    merge(at(p, ls), ll + rl2, ll, buf.ptr, is_less);
                }
                runs[r] = (ls, ll + rl2);
                let mut k = r + 1;
                while k + 1 < n {
                    runs[k] = runs[k + 1];
                    k += 1;
                }
                n -= 1;
            }
        }
        drop(buf);
    }
}

// ---------------------------------------------------------------------------
// Unstable sorting (pattern-defeating quicksort) and selection

/// Sorts `v[..len]` with heapsort (the quicksort fallback).
unsafe fn heapsort<T, F: FnMut(&T, &T) -> bool>(v: *mut T, len: usize, is_less: &mut F) {
    let mut i = len / 2;
    while i > 0 {
        i -= 1;
        sift_down(v, len, i, is_less);
    }
    let mut end = len;
    while end > 1 {
        end -= 1;
        swap(v, 0, end);
        sift_down(v, end, 0, is_less);
    }
}

unsafe fn sift_down<T, F: FnMut(&T, &T) -> bool>(v: *mut T, len: usize, mut node: usize, is_less: &mut F) {
    loop {
        let mut child = 2 * node + 1;
        if child >= len {
            break;
        }
        if child + 1 < len && is_less(&*at(v, child), &*at(v, child + 1)) {
            child += 1;
        }
        if !is_less(&*at(v, node), &*at(v, child)) {
            break;
        }
        swap(v, node, child);
        node = child;
    }
}

/// Sorts the elements at `a`, `b`, `c` (indices) so that v[b] is their median.
#[inline]
unsafe fn sort3<T, F: FnMut(&T, &T) -> bool>(v: *mut T, a: &mut usize, b: &mut usize, c: &mut usize, is_less: &mut F) {
    if is_less(&*at(v, *b), &*at(v, *a)) {
        mem::swap(a, b);
    }
    if is_less(&*at(v, *c), &*at(v, *b)) {
        mem::swap(b, c);
        if is_less(&*at(v, *b), &*at(v, *a)) {
            mem::swap(a, b);
        }
    }
}

/// Chooses a pivot index for `v[..len]` (len > MAX_INSERTION): the median
/// of three, or the pseudo-median of nine for longer slices.
unsafe fn choose_pivot<T, F: FnMut(&T, &T) -> bool>(v: *mut T, len: usize, is_less: &mut F) -> usize {
    let q = len / 4;
    let mut a = q;
    let mut b = q * 2;
    let mut c = q * 3;
    if len >= 64 {
        let (mut a0, mut a1) = (a - 1, a + 1);
        sort3(v, &mut a0, &mut a, &mut a1, is_less);
        let (mut b0, mut b1) = (b - 1, b + 1);
        sort3(v, &mut b0, &mut b, &mut b1, is_less);
        let (mut c0, mut c1) = (c - 1, c + 1);
        sort3(v, &mut c0, &mut c, &mut c1, is_less);
    }
    sort3(v, &mut a, &mut b, &mut c, is_less);
    b
}

/// Partitions `v[..len]` around the pivot `v[0]`: returns `mid` such that
/// afterwards `v[..mid]` are less than the pivot, `v[mid]` is the pivot and
/// `v[mid + 1..]` are not less. Only swaps elements.
unsafe fn partition<T, F: FnMut(&T, &T) -> bool>(v: *mut T, len: usize, is_less: &mut F) -> usize {
    let pivot = v as *const T;
    let mut l = 1;
    let mut r = len;
    loop {
        while l < r && is_less(&*at(v, l), &*pivot) {
            l += 1;
        }
        while l < r && !is_less(&*at(v, r - 1), &*pivot) {
            r -= 1;
        }
        if l >= r {
            break;
        }
        r -= 1;
        swap(v, l, r);
        l += 1;
    }
    swap(v, 0, l - 1);
    l - 1
}

/// Partitions `v[..len]` into the elements equal to the pivot `v[0]` (no
/// element is less than it) and the greater ones; returns the number of
/// elements equal to it (the pivot included), which end up first.
unsafe fn partition_equal<T, F: FnMut(&T, &T) -> bool>(v: *mut T, len: usize, is_less: &mut F) -> usize {
    let pivot = v as *const T;
    let mut l = 1;
    let mut r = len;
    loop {
        while l < r && !is_less(&*pivot, &*at(v, l)) {
            l += 1;
        }
        while l < r && is_less(&*pivot, &*at(v, r - 1)) {
            r -= 1;
        }
        if l >= r {
            break;
        }
        r -= 1;
        swap(v, l, r);
        l += 1;
    }
    l
}

/// Scatters some elements around (after an unbalanced partition) to break
/// adversarial patterns.
unsafe fn break_patterns<T>(v: *mut T, len: usize) {
    let mut seed = len as u64;
    let mut gen = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let mask = len.next_power_of_two() - 1;
    let pos = len / 4 * 2;
    let mut i = 0;
    while i < 3 {
        let mut other = (gen() as usize) & mask;
        if other >= len {
            other -= len;
        }
        swap(v, pos - 1 + i, other);
        i += 1;
    }
}

unsafe fn quicksort<T, F: FnMut(&T, &T) -> bool>(
    mut v: *mut T,
    mut len: usize,
    mut pred: *const T,
    mut limit: u32,
    is_less: &mut F,
) {
    loop {
        if len <= MAX_INSERTION {
            if len >= 2 {
                insertion_sort(v, len, 1, is_less);
            }
            return;
        }
        if limit == 0 {
            heapsort(v, len, is_less);
            return;
        }
        let pi = choose_pivot(v, len, is_less);
        swap(v, 0, pi);
        // the pivot equals the ancestor pivot (all elements are >= it): put
        // the elements equal to it first and continue with the greater ones
        if !pred.is_null() && !is_less(&*pred, &*v) {
            let mid = partition_equal(v, len, is_less);
            v = at(v, mid);
            len -= mid;
            continue;
        }
        let mid = partition(v, len, is_less);
        let left = mid;
        let right = len - mid - 1;
        if left < len / 8 || right < len / 8 {
            limit -= 1;
            if left >= MAX_INSERTION {
                break_patterns(v, left);
            }
            if right >= MAX_INSERTION {
                break_patterns(at(v, mid + 1), right);
            }
        }
        // recurse into the shorter side, loop on the longer one
        if left < right {
            quicksort(v, left, pred, limit, is_less);
            pred = at(v, mid) as *const T;
            v = at(v, mid + 1);
            len = right;
        } else {
            quicksort(at(v, mid + 1), right, at(v, mid) as *const T, limit, is_less);
            len = left;
        }
    }
}

/// Sorts the slice with `is_less` (not stable).
pub(crate) fn unstable_sort<T, F: FnMut(&T, &T) -> bool>(v: &mut [T], is_less: &mut F) {
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
        // an already sorted (or strictly descending) slice
        let run = find_run(p, len, is_less);
        if run == len {
            return;
        }
        let limit = 2 * (usize::BITS - len.leading_zeros());
        quicksort(p, len, crate::ptr::null(), limit, is_less);
    }
}

/// The index of the first minimum of `v[..len]` (or maximum with `max`).
unsafe fn extreme_index<T, F: FnMut(&T, &T) -> bool>(v: *mut T, len: usize, max: bool, is_less: &mut F) -> usize {
    let mut best = 0;
    let mut i = 1;
    while i < len {
        let better = if max { is_less(&*at(v, best), &*at(v, i)) } else { is_less(&*at(v, i), &*at(v, best)) };
        if better {
            best = i;
        }
        i += 1;
    }
    best
}

/// Reorders `v` such that `v[index]` is the element that would be there
/// if it was sorted, the elements before it are not greater and those after
/// it not less (quickselect with a heapsort fallback).
pub(crate) fn select_nth<T, F: FnMut(&T, &T) -> bool>(v: &mut [T], index: usize, is_less: &mut F) {
    let len = v.len();
    if mem::size_of::<T>() == 0 || len < 2 {
        return;
    }
    let p = v.as_mut_ptr();
    unsafe {
        if index == len - 1 {
            let m = extreme_index(p, len, true, is_less);
            swap(p, m, index);
            return;
        }
        if index == 0 {
            let m = extreme_index(p, len, false, is_less);
            swap(p, m, index);
            return;
        }
        let mut v = p;
        let mut len = len;
        let mut index = index;
        let mut pred: *const T = crate::ptr::null();
        let mut limit = 16;
        loop {
            if len <= 16 {
                if len >= 2 {
                    insertion_sort(v, len, 1, is_less);
                }
                return;
            }
            if limit == 0 {
                heapsort(v, len, is_less);
                return;
            }
            limit -= 1;
            let pi = choose_pivot(v, len, is_less);
            swap(v, 0, pi);
            if !pred.is_null() && !is_less(&*pred, &*v) {
                let mid = partition_equal(v, len, is_less);
                if index < mid {
                    return; // all equal to the pivot
                }
                v = at(v, mid);
                len -= mid;
                index -= mid;
                pred = crate::ptr::null();
                continue;
            }
            let mid = partition(v, len, is_less);
            if mid == index {
                return;
            }
            if index < mid {
                len = mid;
            } else {
                pred = at(v, mid) as *const T;
                v = at(v, mid + 1);
                len -= mid + 1;
                index -= mid + 1;
            }
        }
    }
}
