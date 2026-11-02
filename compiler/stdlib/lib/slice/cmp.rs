//! Comparison traits for slices and arrays.

use crate::cmp::Ordering;
use crate::intrinsics;

impl<A, B> PartialEq<[B]> for [A]
where
    A: PartialEq<B>,
{
    fn eq(&self, other: &[B]) -> bool {
        let n = self.len();
        if n != other.len() {
            return false;
        }
        let mut i = 0;
        while i < n {
            if self[i] != other[i] {
                return false;
            }
            i += 1;
        }
        true
    }
}

impl<T: Eq> Eq for [T] {}

impl<T: PartialOrd> PartialOrd for [T] {
    fn partial_cmp(&self, other: &[T]) -> Option<Ordering> {
        let l = if self.len() < other.len() { self.len() } else { other.len() };
        let mut i = 0;
        while i < l {
            match self[i].partial_cmp(&other[i]) {
                Some(Ordering::Equal) => (),
                non_eq => return non_eq,
            }
            i += 1;
        }
        self.len().partial_cmp(&other.len())
    }
}

impl<T: Ord> Ord for [T] {
    fn cmp(&self, other: &[T]) -> Ordering {
        let l = if self.len() < other.len() { self.len() } else { other.len() };
        let mut i = 0;
        while i < l {
            match self[i].cmp(&other[i]) {
                Ordering::Equal => (),
                non_eq => return non_eq,
            }
            i += 1;
        }
        self.len().cmp(&other.len())
    }
}

/// Byte-slice comparison helper (memcmp based).
#[inline]
pub(crate) fn compare_u8(a: &[u8], b: &[u8]) -> Ordering {
    let n = if a.len() < b.len() { a.len() } else { b.len() };
    let r = unsafe { intrinsics::compare_bytes(a.as_ptr(), b.as_ptr(), n) };
    if r < 0 {
        Ordering::Less
    } else if r > 0 {
        Ordering::Greater
    } else {
        a.len().cmp(&b.len())
    }
}
