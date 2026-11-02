//! Stable merge sort used by all slice sorting methods.

use crate::intrinsics;

const INSERTION_THRESHOLD: usize = 20;

/// Sorts `v[0..len]` (raw pointer `v`) with insertion sort.
unsafe fn insertion_sort<T, F: FnMut(&T, &T) -> bool>(v: *mut T, len: usize, is_less: &mut F) {
    let mut i = 1;
    while i < len {
        let p_i = intrinsics::ptr_add_mut(v, i);
        if is_less(&*p_i, &*intrinsics::ptr_add_mut(v, i - 1)) {
            let tmp = intrinsics::read(p_i as *const T);
            let mut j = i;
            loop {
                intrinsics::copy_nonoverlapping(
                    intrinsics::ptr_add_mut(v, j - 1) as *const T,
                    intrinsics::ptr_add_mut(v, j),
                    1,
                );
                j -= 1;
                if j == 0 || !is_less(&tmp, &*intrinsics::ptr_add_mut(v, j - 1)) {
                    break;
                }
            }
            intrinsics::write(intrinsics::ptr_add_mut(v, j), tmp);
        }
        i += 1;
    }
}

/// Merges the sorted runs `v[0..mid]` and `v[mid..len]` using `buf` (room
/// for at least `mid` elements).
unsafe fn merge<T, F: FnMut(&T, &T) -> bool>(v: *mut T, len: usize, mid: usize, buf: *mut T, is_less: &mut F) {
    intrinsics::copy_nonoverlapping(v as *const T, buf, mid);
    let mut i = 0; // index into buf (left run)
    let mut j = mid; // index into v (right run)
    let mut k = 0; // destination index in v
    while i < mid && j < len {
        let right = intrinsics::ptr_add_mut(v, j);
        let left = intrinsics::ptr_add_mut(buf, i);
        if is_less(&*right, &*left) {
            intrinsics::copy_nonoverlapping(right as *const T, intrinsics::ptr_add_mut(v, k), 1);
            j += 1;
        } else {
            intrinsics::copy_nonoverlapping(left as *const T, intrinsics::ptr_add_mut(v, k), 1);
            i += 1;
        }
        k += 1;
    }
    if i < mid {
        intrinsics::copy_nonoverlapping(intrinsics::ptr_add_mut(buf, i) as *const T, intrinsics::ptr_add_mut(v, k), mid - i);
    }
}

unsafe fn merge_sort_rec<T, F: FnMut(&T, &T) -> bool>(v: *mut T, len: usize, buf: *mut T, is_less: &mut F) {
    if len <= INSERTION_THRESHOLD {
        insertion_sort(v, len, is_less);
        return;
    }
    let mid = len / 2;
    merge_sort_rec(v, mid, buf, is_less);
    merge_sort_rec(intrinsics::ptr_add_mut(v, mid), len - mid, buf, is_less);
    // Already in order?
    if !is_less(&*intrinsics::ptr_add_mut(v, mid), &*intrinsics::ptr_add_mut(v, mid - 1)) {
        return;
    }
    merge(v, len, mid, buf, is_less);
}

/// Sorts the slice stably with the strict-weak-order predicate `is_less`.
pub(crate) fn merge_sort<T, F: FnMut(&T, &T) -> bool>(v: &mut [T], is_less: &mut F) {
    let len = v.len();
    if len < 2 || crate::mem::size_of::<T>() == 0 {
        return;
    }
    let p = v.as_mut_ptr();
    unsafe {
        if len <= INSERTION_THRESHOLD {
            insertion_sort(p, len, is_less);
            return;
        }
        let buf_len = len / 2 + 1;
        let buf = crate::alloc::alloc_array::<T>(buf_len);
        merge_sort_rec(p, len, buf, is_less);
        crate::alloc::dealloc_array(buf, buf_len);
    }
}
