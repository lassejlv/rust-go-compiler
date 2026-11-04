//! std's unstable sort (ipnsort by Lukas Bergdoll and Orson Peters) and
//! introselect (`select_nth_unstable`), ported from core::slice::sort so that
//! the order of equal elements (and the comparisons made) match std exactly.
//!
//! std chooses its small-sort by specializing on `Freeze` and `Copy`; here the
//! `type_is_freeze` / `type_is_copy` intrinsics make the same choice.

use crate::intrinsics;
use crate::mem::{self, ManuallyDrop, MaybeUninit};
use crate::ptr;
use crate::slice;

// ---------------------------------------------------------------------------
// Entry points

/// `slice::sort_unstable*`.
pub(crate) fn sort<T, F: FnMut(&T, &T) -> bool>(v: &mut [T], is_less: &mut F) {
    if mem::size_of::<T>() == 0 {
        return;
    }
    let len = v.len();
    if len < 2 {
        return;
    }
    const MAX_LEN_ALWAYS_INSERTION_SORT: usize = 20;
    if len <= MAX_LEN_ALWAYS_INSERTION_SORT {
        insertion_sort_shift_left(v, 1, is_less);
        return;
    }
    ipnsort(v, is_less);
}

fn ipnsort<T, F: FnMut(&T, &T) -> bool>(v: &mut [T], is_less: &mut F) {
    let len = v.len();
    let (run_len, was_reversed) = find_existing_run(v, is_less);
    if run_len == len {
        if was_reversed {
            v.reverse();
        }
        return;
    }
    // Limit the number of imbalanced partitions to `2 * floor(log2(len))`.
    let limit = 2 * (len | 1).ilog2();
    quicksort(v, None, limit, is_less);
}

fn find_existing_run<T, F: FnMut(&T, &T) -> bool>(v: &[T], is_less: &mut F) -> (usize, bool) {
    let len = v.len();
    if len < 2 {
        return (len, false);
    }
    unsafe {
        let mut run_len = 2;
        let strictly_descending = is_less(v.get_unchecked(1), v.get_unchecked(0));
        if strictly_descending {
            while run_len < len && is_less(v.get_unchecked(run_len), v.get_unchecked(run_len - 1)) {
                run_len += 1;
            }
        } else {
            while run_len < len && !is_less(v.get_unchecked(run_len), v.get_unchecked(run_len - 1)) {
                run_len += 1;
            }
        }
        (run_len, strictly_descending)
    }
}

// ---------------------------------------------------------------------------
// Quicksort

fn quicksort<'a, T, F: FnMut(&T, &T) -> bool>(
    mut v: &'a mut [T],
    mut ancestor_pivot: Option<&'a T>,
    mut limit: u32,
    is_less: &mut F,
) {
    loop {
        if v.len() <= small_sort_threshold::<T>() {
            small_sort(v, is_less);
            return;
        }

        if limit == 0 {
            heapsort(v, is_less);
            return;
        }

        limit -= 1;

        let pivot_pos = choose_pivot(v, is_less);

        if let Some(p) = ancestor_pivot {
            if !is_less(p, &v[pivot_pos]) {
                let num_lt = partition(v, pivot_pos, &mut |a: &T, b: &T| !is_less(b, a));
                v = &mut v[(num_lt + 1)..];
                ancestor_pivot = None;
                continue;
            }
        }

        let num_lt = partition(v, pivot_pos, is_less);

        let (left, right) = v.split_at_mut(num_lt);
        let (pivot, right) = right.split_at_mut(1);
        let pivot = &pivot[0];

        quicksort(left, ancestor_pivot, limit, is_less);

        v = right;
        ancestor_pivot = Some(pivot);
    }
}

/// Moves the elements less than `v[pivot]` to the front, followed by the pivot;
/// returns the number of elements less than the pivot.
fn partition<T, F: FnMut(&T, &T) -> bool>(v: &mut [T], pivot: usize, is_less: &mut F) -> usize {
    let len = v.len();
    if len == 0 {
        return 0;
    }
    if pivot >= len {
        intrinsics::abort();
    }
    v.swap(0, pivot);
    let (pivot, v_without_pivot) = v.split_at_mut(1);
    let pivot = &mut pivot[0];

    const MAX_BRANCHLESS_PARTITION_SIZE: usize = 96;
    let num_lt = if mem::size_of::<T>() <= MAX_BRANCHLESS_PARTITION_SIZE {
        partition_lomuto_branchless_cyclic(v_without_pivot, pivot, is_less)
    } else {
        partition_hoare_branchy_cyclic(v_without_pivot, pivot, is_less)
    };

    if num_lt >= len {
        intrinsics::abort();
    }
    v.swap(0, num_lt);
    num_lt
}

fn partition_hoare_branchy_cyclic<T, F: FnMut(&T, &T) -> bool>(v: &mut [T], pivot: &T, is_less: &mut F) -> usize {
    let len = v.len();
    if len == 0 {
        return 0;
    }

    let mut gap_opt: Option<GapGuard<T>> = None;

    unsafe {
        let v_base = v.as_mut_ptr();

        let mut left = v_base;
        let mut right = v_base.add(len);

        loop {
            // Find the first element greater than the pivot.
            while left < right && is_less(&*left, pivot) {
                left = left.add(1);
            }

            // Find the last element equal to the pivot.
            loop {
                right = right.sub(1);
                if left >= right || is_less(&*right, pivot) {
                    break;
                }
            }

            if left >= right {
                break;
            }

            let is_first_swap_pair = gap_opt.is_none();

            if is_first_swap_pair {
                gap_opt = Some(GapGuard { pos: right, value: ManuallyDrop::new(ptr::read(left)) });
            }

            let gap = gap_opt.as_mut().unwrap();

            if !is_first_swap_pair {
                ptr::copy_nonoverlapping(left, gap.pos, 1);
            }
            gap.pos = right;
            ptr::copy_nonoverlapping(right, left, 1);

            left = left.add(1);
        }

        let n = left.offset_from(v_base) as usize;
        drop(gap_opt);
        n
    }
}

fn partition_lomuto_branchless_cyclic<T, F: FnMut(&T, &T) -> bool>(v: &mut [T], pivot: &T, is_less: &mut F) -> usize {
    let len = v.len();
    let v_base = v.as_mut_ptr();

    if len == 0 {
        return 0;
    }

    // std runs the loop body through a closure with a 2x manual unroll; the
    // sequence of steps (and comparisons) is the same as this single loop.
    unsafe {
        let mut gap_value = ManuallyDrop::new(ptr::read(v_base));
        let gap_value_ptr = &mut *gap_value as *mut T;

        let mut num_lt: usize = 0;
        let mut right = v_base.add(1);
        let mut gap = GapGuardRaw { pos: v_base, value: gap_value_ptr };

        let end = v_base.add(len);
        loop {
            let is_done = right == end;
            if is_done {
                right = gap_value_ptr;
            }

            let right_is_lt = is_less(&*right, pivot);
            let left = v_base.add(num_lt);

            ptr::copy(left, gap.pos, 1);
            ptr::copy_nonoverlapping(right, left, 1);

            gap.pos = right;
            num_lt += right_is_lt as usize;

            right = right.add(1);

            if is_done {
                mem::forget(gap);
                break;
            }
        }

        num_lt
    }
}

struct GapGuard<T> {
    pos: *mut T,
    value: ManuallyDrop<T>,
}

impl<T> Drop for GapGuard<T> {
    fn drop(&mut self) {
        unsafe {
            ptr::copy_nonoverlapping(&*self.value as *const T, self.pos, 1);
        }
    }
}

struct GapGuardRaw<T> {
    pos: *mut T,
    value: *mut T,
}

impl<T> Drop for GapGuardRaw<T> {
    fn drop(&mut self) {
        unsafe {
            ptr::copy_nonoverlapping(self.value as *const T, self.pos, 1);
        }
    }
}

// ---------------------------------------------------------------------------
// Heapsort fallback

fn heapsort<T, F: FnMut(&T, &T) -> bool>(v: &mut [T], is_less: &mut F) {
    let len = v.len();
    let mut i = len + len / 2;
    while i > 0 {
        i -= 1;
        let sift_idx = if i >= len {
            i - len
        } else {
            v.swap(0, i);
            0
        };
        let end = if i < len { i } else { len };
        unsafe {
            sift_down(&mut v[..end], sift_idx, is_less);
        }
    }
}

unsafe fn sift_down<T, F: FnMut(&T, &T) -> bool>(v: &mut [T], mut node: usize, is_less: &mut F) {
    let len = v.len();
    let v_base = v.as_mut_ptr();
    loop {
        let mut child = 2 * node + 1;
        if child >= len {
            break;
        }
        if child + 1 < len {
            child += is_less(&*v_base.add(child), &*v_base.add(child + 1)) as usize;
        }
        if !is_less(&*v_base.add(node), &*v_base.add(child)) {
            break;
        }
        ptr::swap_nonoverlapping(v_base.add(node), v_base.add(child), 1);
        node = child;
    }
}

// ---------------------------------------------------------------------------
// Pivot selection

const PSEUDO_MEDIAN_REC_THRESHOLD: usize = 64;

fn choose_pivot<T, F: FnMut(&T, &T) -> bool>(v: &[T], is_less: &mut F) -> usize {
    let len = v.len();
    if len < 8 {
        intrinsics::abort();
    }
    unsafe {
        let v_base = v.as_ptr();
        let len_div_8 = len / 8;

        let a = v_base;
        let b = v_base.add(len_div_8 * 4);
        let c = v_base.add(len_div_8 * 7);

        if len < PSEUDO_MEDIAN_REC_THRESHOLD {
            median3(&*a, &*b, &*c, is_less).offset_from(v_base) as usize
        } else {
            median3_rec(a, b, c, len_div_8, is_less).offset_from(v_base) as usize
        }
    }
}

unsafe fn median3_rec<T, F: FnMut(&T, &T) -> bool>(
    mut a: *const T,
    mut b: *const T,
    mut c: *const T,
    n: usize,
    is_less: &mut F,
) -> *const T {
    if n * 8 >= PSEUDO_MEDIAN_REC_THRESHOLD {
        let n8 = n / 8;
        a = median3_rec(a, a.add(n8 * 4), a.add(n8 * 7), n8, is_less);
        b = median3_rec(b, b.add(n8 * 4), b.add(n8 * 7), n8, is_less);
        c = median3_rec(c, c.add(n8 * 4), c.add(n8 * 7), n8, is_less);
    }
    median3(&*a, &*b, &*c, is_less)
}

fn median3<T, F: FnMut(&T, &T) -> bool>(a: &T, b: &T, c: &T, is_less: &mut F) -> *const T {
    let x = is_less(a, b);
    let y = is_less(a, c);
    if x == y {
        let z = is_less(b, c);
        if z ^ x { c as *const T } else { b as *const T }
    } else {
        a as *const T
    }
}

// ---------------------------------------------------------------------------
// Small sorts

const SMALL_SORT_FALLBACK_THRESHOLD: usize = 16;
const SMALL_SORT_GENERAL_THRESHOLD: usize = 32;
const SMALL_SORT_GENERAL_SCRATCH_LEN: usize = SMALL_SORT_GENERAL_THRESHOLD + 16;
const SMALL_SORT_NETWORK_THRESHOLD: usize = 32;
const SMALL_SORT_NETWORK_SCRATCH_LEN: usize = SMALL_SORT_NETWORK_THRESHOLD;
const MAX_STACK_ARRAY_SIZE: usize = 4096;

/// Which small-sort std's specialization picks for `T`:
/// 0 = insertion sort fallback, 1 = general (sort4/8 + merge), 2 = sorting network.
#[inline]
fn small_sort_kind<T>() -> u8 {
    if !intrinsics::type_is_freeze::<T>() {
        return 0;
    }
    let size = mem::size_of::<T>();
    if intrinsics::type_is_copy::<T>() && size <= 8 && size * SMALL_SORT_NETWORK_SCRATCH_LEN <= MAX_STACK_ARRAY_SIZE {
        2
    } else if size * SMALL_SORT_GENERAL_SCRATCH_LEN <= MAX_STACK_ARRAY_SIZE {
        1
    } else {
        0
    }
}

#[inline]
fn small_sort_threshold<T>() -> usize {
    match small_sort_kind::<T>() {
        2 => SMALL_SORT_NETWORK_THRESHOLD,
        1 => SMALL_SORT_GENERAL_THRESHOLD,
        _ => SMALL_SORT_FALLBACK_THRESHOLD,
    }
}

fn small_sort<T, F: FnMut(&T, &T) -> bool>(v: &mut [T], is_less: &mut F) {
    match small_sort_kind::<T>() {
        2 => small_sort_network(v, is_less),
        1 => small_sort_general(v, is_less),
        _ => {
            if v.len() >= 2 {
                insertion_sort_shift_left(v, 1, is_less);
            }
        }
    }
}

fn small_sort_general<T, F: FnMut(&T, &T) -> bool>(v: &mut [T], is_less: &mut F) {
    let mut stack_array = MaybeUninit::<[T; SMALL_SORT_GENERAL_SCRATCH_LEN]>::uninit();
    let scratch = unsafe {
        slice::from_raw_parts_mut(stack_array.as_mut_ptr() as *mut MaybeUninit<T>, SMALL_SORT_GENERAL_SCRATCH_LEN)
    };
    small_sort_general_with_scratch(v, scratch, is_less);
}

fn small_sort_general_with_scratch<T, F: FnMut(&T, &T) -> bool>(
    v: &mut [T],
    scratch: &mut [MaybeUninit<T>],
    is_less: &mut F,
) {
    let len = v.len();
    if len < 2 {
        return;
    }
    if scratch.len() < len + 16 {
        intrinsics::abort();
    }

    let v_base = v.as_mut_ptr();
    let len_div_2 = len / 2;

    unsafe {
        let scratch_base = scratch.as_mut_ptr() as *mut T;

        let presorted_len = if mem::size_of::<T>() <= 16 && len >= 16 {
            sort8_stable(v_base, scratch_base, scratch_base.add(len), is_less);
            sort8_stable(v_base.add(len_div_2), scratch_base.add(len_div_2), scratch_base.add(len + 8), is_less);
            8
        } else if len >= 8 {
            sort4_stable(v_base, scratch_base, is_less);
            sort4_stable(v_base.add(len_div_2), scratch_base.add(len_div_2), is_less);
            4
        } else {
            ptr::copy_nonoverlapping(v_base, scratch_base, 1);
            ptr::copy_nonoverlapping(v_base.add(len_div_2), scratch_base.add(len_div_2), 1);
            1
        };

        for offset in [0, len_div_2] {
            let src = v_base.add(offset);
            let dst = scratch_base.add(offset);
            let desired_len = if offset == 0 { len_div_2 } else { len - len_div_2 };

            let mut i = presorted_len;
            while i < desired_len {
                ptr::copy_nonoverlapping(src.add(i), dst.add(i), 1);
                insert_tail(dst, dst.add(i), is_less);
                i += 1;
            }
        }

        let drop_guard = CopyOnDrop { src: scratch_base as *const T, dst: v_base, len };

        bidirectional_merge(slice::from_raw_parts(drop_guard.src, drop_guard.len), drop_guard.dst, is_less);
        mem::forget(drop_guard);
    }
}

struct CopyOnDrop<T> {
    src: *const T,
    dst: *mut T,
    len: usize,
}

impl<T> Drop for CopyOnDrop<T> {
    fn drop(&mut self) {
        unsafe {
            ptr::copy_nonoverlapping(self.src, self.dst, self.len);
        }
    }
}

fn small_sort_network<T, F: FnMut(&T, &T) -> bool>(v: &mut [T], is_less: &mut F) {
    let len = v.len();
    if len < 2 {
        return;
    }
    if len > SMALL_SORT_NETWORK_SCRATCH_LEN {
        intrinsics::abort();
    }

    let mut stack_array = MaybeUninit::<[T; SMALL_SORT_NETWORK_SCRATCH_LEN]>::uninit();

    let len_div_2 = len / 2;
    let no_merge = len < 18;

    let v_base = v.as_mut_ptr();
    let initial_region_len = if no_merge { len } else { len_div_2 };
    let mut region = unsafe { slice::from_raw_parts_mut(v_base, initial_region_len) };

    loop {
        let presorted_len = if region.len() >= 13 {
            sort13_optimal(region, is_less);
            13
        } else if region.len() >= 9 {
            sort9_optimal(region, is_less);
            9
        } else {
            1
        };

        insertion_sort_shift_left(region, presorted_len, is_less);

        if no_merge {
            return;
        }

        if region.as_ptr() != v_base as *const T {
            break;
        }

        unsafe { region = slice::from_raw_parts_mut(v_base.add(len_div_2), len - len_div_2) };
    }

    unsafe {
        let scratch_base = stack_array.as_mut_ptr() as *mut T;
        bidirectional_merge(slice::from_raw_parts_mut(v_base, len), scratch_base, is_less);
        ptr::copy_nonoverlapping(scratch_base as *const T, v_base, len);
    }
}

unsafe fn swap_if_less<T, F: FnMut(&T, &T) -> bool>(v_base: *mut T, a_pos: usize, b_pos: usize, is_less: &mut F) {
    let v_a = v_base.add(a_pos);
    let v_b = v_base.add(b_pos);

    let should_swap = is_less(&*v_b, &*v_a);

    let v_a_swap = if should_swap { v_b } else { v_a };
    let v_b_swap = if should_swap { v_a } else { v_b };

    let v_b_swap_tmp = ManuallyDrop::new(ptr::read(v_b_swap));
    ptr::copy(v_a_swap, v_a, 1);
    ptr::copy_nonoverlapping(&*v_b_swap_tmp as *const T, v_b, 1);
}

fn sort9_optimal<T, F: FnMut(&T, &T) -> bool>(v: &mut [T], is_less: &mut F) {
    if v.len() < 9 {
        intrinsics::abort();
    }
    let v_base = v.as_mut_ptr();
    static NET: [(usize, usize); 25] = [
        (0, 3), (1, 7), (2, 5), (4, 8), (0, 7), (2, 4), (3, 8), (5, 6), (0, 2), (1, 3), (4, 5), (7, 8), (1, 4),
        (3, 6), (5, 7), (0, 1), (2, 4), (3, 5), (6, 8), (2, 3), (4, 5), (6, 7), (1, 2), (3, 4), (5, 6),
    ];
    let mut i = 0;
    while i < NET.len() {
        unsafe { swap_if_less(v_base, NET[i].0, NET[i].1, is_less) };
        i += 1;
    }
}

fn sort13_optimal<T, F: FnMut(&T, &T) -> bool>(v: &mut [T], is_less: &mut F) {
    if v.len() < 13 {
        intrinsics::abort();
    }
    let v_base = v.as_mut_ptr();
    static NET: [(usize, usize); 45] = [
        (0, 12), (1, 10), (2, 9), (3, 7), (5, 11), (6, 8), (1, 6), (2, 3), (4, 11), (7, 9), (8, 10), (0, 4),
        (1, 2), (3, 6), (7, 8), (9, 10), (11, 12), (4, 6), (5, 9), (8, 11), (10, 12), (0, 5), (3, 8), (4, 7),
        (6, 11), (9, 10), (0, 1), (2, 5), (6, 9), (7, 8), (10, 11), (1, 3), (2, 4), (5, 6), (9, 10), (1, 2),
        (3, 4), (5, 7), (6, 8), (2, 3), (4, 5), (6, 7), (8, 9), (3, 4), (5, 6),
    ];
    let mut i = 0;
    while i < NET.len() {
        unsafe { swap_if_less(v_base, NET[i].0, NET[i].1, is_less) };
        i += 1;
    }
}

/// Sorts range [begin, tail] assuming [begin, tail) is already sorted.
unsafe fn insert_tail<T, F: FnMut(&T, &T) -> bool>(begin: *mut T, tail: *mut T, is_less: &mut F) {
    let mut sift = tail.sub(1);
    if !is_less(&*tail, &*sift) {
        return;
    }

    let tmp = ManuallyDrop::new(tail.read());
    let mut gap_guard = CopyOnDrop { src: &*tmp as *const T, dst: tail, len: 1 };

    loop {
        ptr::copy_nonoverlapping(sift as *const T, gap_guard.dst, 1);
        gap_guard.dst = sift;

        if sift == begin {
            break;
        }

        sift = sift.sub(1);
        if !is_less(&tmp, &*sift) {
            break;
        }
    }
}

/// Sorts `v` assuming `v[..offset]` is already sorted.
pub(crate) fn insertion_sort_shift_left<T, F: FnMut(&T, &T) -> bool>(v: &mut [T], offset: usize, is_less: &mut F) {
    let len = v.len();
    if offset == 0 || offset > len {
        intrinsics::abort();
    }
    unsafe {
        let v_base = v.as_mut_ptr();
        let v_end = v_base.add(len);
        let mut tail = v_base.add(offset);
        while tail != v_end {
            insert_tail(v_base, tail, is_less);
            tail = tail.add(1);
        }
    }
}

unsafe fn sort4_stable<T, F: FnMut(&T, &T) -> bool>(v_base: *const T, dst: *mut T, is_less: &mut F) {
    let c1 = is_less(&*v_base.add(1), &*v_base);
    let c2 = is_less(&*v_base.add(3), &*v_base.add(2));
    let a = v_base.add(c1 as usize);
    let b = v_base.add(!c1 as usize);
    let c = v_base.add(2 + c2 as usize);
    let d = v_base.add(2 + (!c2 as usize));

    let c3 = is_less(&*c, &*a);
    let c4 = is_less(&*d, &*b);
    let min = if c3 { c } else { a };
    let max = if c4 { b } else { d };
    let unknown_left = if c3 { a } else if c4 { c } else { b };
    let unknown_right = if c4 { d } else if c3 { b } else { c };

    let c5 = is_less(&*unknown_right, &*unknown_left);
    let lo = if c5 { unknown_right } else { unknown_left };
    let hi = if c5 { unknown_left } else { unknown_right };

    ptr::copy_nonoverlapping(min, dst, 1);
    ptr::copy_nonoverlapping(lo, dst.add(1), 1);
    ptr::copy_nonoverlapping(hi, dst.add(2), 1);
    ptr::copy_nonoverlapping(max, dst.add(3), 1);
}

unsafe fn sort8_stable<T, F: FnMut(&T, &T) -> bool>(v_base: *mut T, dst: *mut T, scratch_base: *mut T, is_less: &mut F) {
    sort4_stable(v_base as *const T, scratch_base, is_less);
    sort4_stable(v_base.add(4) as *const T, scratch_base.add(4), is_less);
    bidirectional_merge(slice::from_raw_parts(scratch_base as *const T, 8), dst, is_less);
}

/// Merges `v` (two sorted halves `v[..len / 2]` and `v[len / 2..]`) into `dst`,
/// from both ends at once.
unsafe fn bidirectional_merge<T, F: FnMut(&T, &T) -> bool>(v: &[T], dst: *mut T, is_less: &mut F) {
    let len = v.len();
    let src = v.as_ptr();
    let len_div_2 = len / 2;

    let mut left = src;
    let mut right = src.add(len_div_2);
    let mut dst = dst;

    let mut left_rev = src.add(len_div_2 - 1);
    let mut right_rev = src.add(len - 1);
    let mut dst_rev = dst.add(len - 1);

    let mut k = 0;
    while k < len_div_2 {
        // merge up
        let is_l = !is_less(&*right, &*left);
        let s = if is_l { left } else { right };
        ptr::copy_nonoverlapping(s, dst, 1);
        right = right.add(!is_l as usize);
        left = left.add(is_l as usize);
        dst = dst.add(1);

        // merge down
        let is_l = !is_less(&*right_rev, &*left_rev);
        let s = if is_l { right_rev } else { left_rev };
        ptr::copy_nonoverlapping(s, dst_rev, 1);
        right_rev = right_rev.wrapping_sub(is_l as usize);
        left_rev = left_rev.wrapping_sub(!is_l as usize);
        dst_rev = dst_rev.sub(1);

        k += 1;
    }

    let left_end = left_rev.wrapping_add(1);
    let right_end = right_rev.wrapping_add(1);

    if len % 2 != 0 {
        let left_nonempty = left < left_end;
        let last_src = if left_nonempty { left } else { right };
        ptr::copy_nonoverlapping(last_src, dst, 1);
        left = left.add(left_nonempty as usize);
        right = right.add((!left_nonempty) as usize);
    }

    if left != left_end || right != right_end {
        panic_on_ord_violation();
    }
}

#[inline(never)]
#[track_caller]
fn panic_on_ord_violation() -> ! {
    panic!("user-provided comparison function does not correctly implement a total order");
}

// ---------------------------------------------------------------------------
// select_nth_unstable (introselect)

/// Reorders the slice such that the element at `index` is at its final sorted position.
pub(crate) fn partition_at_index<T, F: FnMut(&T, &T) -> bool>(
    v: &mut [T],
    index: usize,
    mut is_less: F,
) -> (&mut [T], &mut T, &mut [T]) {
    let len = v.len();

    if index >= len {
        panic!("partition_at_index index {} greater than length of slice {}", index, len);
    }

    if mem::size_of::<T>() == 0 {
        // Sorting has no meaningful behavior on zero-sized types. Do nothing.
    } else if index == len - 1 {
        let max_idx = max_index(v, &mut is_less).unwrap();
        v.swap(max_idx, index);
    } else if index == 0 {
        let min_idx = min_index(v, &mut is_less).unwrap();
        v.swap(min_idx, index);
    } else {
        partition_at_index_loop(v, index, None, &mut is_less);
    }

    let (left, right) = v.split_at_mut(index);
    let (pivot, right) = right.split_at_mut(1);
    let pivot = &mut pivot[0];
    (left, pivot, right)
}

const INSERTION_SORT_THRESHOLD: usize = 16;

fn partition_at_index_loop<'a, T, F: FnMut(&T, &T) -> bool>(
    mut v: &'a mut [T],
    mut index: usize,
    mut ancestor_pivot: Option<&'a T>,
    is_less: &mut F,
) {
    let mut limit = 16;

    loop {
        if v.len() <= INSERTION_SORT_THRESHOLD {
            if v.len() >= 2 {
                insertion_sort_shift_left(v, 1, is_less);
            }
            return;
        }

        if limit == 0 {
            median_of_medians(v, is_less, index);
            return;
        }

        limit -= 1;

        let pivot_pos = choose_pivot(v, is_less);

        if let Some(p) = ancestor_pivot {
            let pivot = &v[pivot_pos];

            if !is_less(p, pivot) {
                let num_lt = partition(v, pivot_pos, &mut |a: &T, b: &T| !is_less(b, a));
                let mid = num_lt + 1;
                if mid > index {
                    return;
                }
                v = &mut v[mid..];
                index = index - mid;
                ancestor_pivot = None;
                continue;
            }
        }

        let mid = partition(v, pivot_pos, is_less);

        let (left, right) = v.split_at_mut(mid);
        let (pivot, right) = right.split_at_mut(1);
        let pivot = &pivot[0];

        if mid < index {
            v = right;
            index = index - mid - 1;
            ancestor_pivot = Some(pivot);
        } else if mid > index {
            v = left;
        } else {
            return;
        }
    }
}

fn min_index<T, F: FnMut(&T, &T) -> bool>(slice: &[T], is_less: &mut F) -> Option<usize> {
    slice.iter().enumerate().reduce(|acc, t| if is_less(t.1, acc.1) { t } else { acc }).map(|(i, _)| i)
}

fn max_index<T, F: FnMut(&T, &T) -> bool>(slice: &[T], is_less: &mut F) -> Option<usize> {
    slice.iter().enumerate().reduce(|acc, t| if is_less(acc.1, t.1) { t } else { acc }).map(|(i, _)| i)
}

fn median_of_medians<T, F: FnMut(&T, &T) -> bool>(mut v: &mut [T], is_less: &mut F, mut k: usize) {
    loop {
        if v.len() <= INSERTION_SORT_THRESHOLD {
            if v.len() >= 2 {
                insertion_sort_shift_left(v, 1, is_less);
            }
            return;
        }

        if k == v.len() - 1 {
            let max_idx = max_index(v, is_less).unwrap();
            v.swap(max_idx, k);
            return;
        } else if k == 0 {
            let min_idx = min_index(v, is_less).unwrap();
            v.swap(min_idx, k);
            return;
        }

        let p = median_of_ninthers(v, is_less);

        if p == k {
            return;
        } else if p > k {
            v = &mut v[..p];
        } else {
            v = &mut v[p + 1..];
            k -= p + 1;
        }
    }
}

fn median_of_ninthers<T, F: FnMut(&T, &T) -> bool>(v: &mut [T], is_less: &mut F) -> usize {
    let frac = if v.len() <= 1024 {
        v.len() / 12
    } else if v.len() <= 128_usize.saturating_mul(1024) {
        v.len() / 64
    } else {
        v.len() / 1024
    };

    let pivot = frac / 2;
    let lo = v.len() / 2 - pivot;
    let hi = frac + lo;
    let gap = (v.len() - 9 * frac) / 4;
    let mut a = lo - 4 * frac - gap;
    let mut b = hi + gap;
    for i in lo..hi {
        ninther(v, is_less, a, i - frac, b, a + 1, i, b + 1, a + 2, i + frac, b + 2);
        a += 3;
        b += 3;
    }

    median_of_medians(&mut v[lo..lo + frac], is_less, pivot);

    partition(v, lo + pivot, is_less)
}

fn ninther<T, F: FnMut(&T, &T) -> bool>(
    v: &mut [T],
    is_less: &mut F,
    a: usize,
    mut b: usize,
    c: usize,
    mut d: usize,
    e: usize,
    mut f: usize,
    g: usize,
    mut h: usize,
    i: usize,
) {
    b = median_idx(v, is_less, a, b, c);
    h = median_idx(v, is_less, g, h, i);
    if is_less(&v[h], &v[b]) {
        mem::swap(&mut b, &mut h);
    }
    if is_less(&v[f], &v[d]) {
        mem::swap(&mut d, &mut f);
    }
    if is_less(&v[e], &v[d]) {
        // do nothing
    } else if is_less(&v[f], &v[e]) {
        d = f;
    } else {
        if is_less(&v[e], &v[b]) {
            v.swap(e, b);
        } else if is_less(&v[h], &v[e]) {
            v.swap(e, h);
        }
        return;
    }
    if is_less(&v[d], &v[b]) {
        d = b;
    } else if is_less(&v[h], &v[d]) {
        d = h;
    }

    v.swap(d, e);
}

fn median_idx<T, F: FnMut(&T, &T) -> bool>(v: &[T], is_less: &mut F, mut a: usize, b: usize, mut c: usize) -> usize {
    if is_less(&v[c], &v[a]) {
        mem::swap(&mut a, &mut c);
    }
    if is_less(&v[c], &v[b]) {
        return c;
    }
    if is_less(&v[b], &v[a]) {
        return a;
    }
    b
}
