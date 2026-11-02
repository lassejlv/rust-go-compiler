//! Ranges as iterators.

use super::traits::{DoubleEndedIterator, ExactSizeIterator, FusedIterator, Iterator, TrustedLen};
use crate::mem;
use crate::ops::{Range, RangeFrom, RangeInclusive};

/// Objects that have a notion of *successor* and *predecessor* operations.
pub trait Step: Clone + PartialOrd + Sized {
    /// Returns the bounds on the number of *successor* steps required to get from `start` to `end`.
    fn steps_between(start: &Self, end: &Self) -> (usize, Option<usize>);

    /// Returns the value that would be obtained by taking the *successor* of `self` `count` times.
    fn forward_checked(start: Self, count: usize) -> Option<Self>;

    /// Returns the value that would be obtained by taking the *predecessor* of `self` `count` times.
    fn backward_checked(start: Self, count: usize) -> Option<Self>;

    /// Like `forward_checked`, but panics on overflow.
    fn forward(start: Self, count: usize) -> Self {
        match Step::forward_checked(start, count) {
            Some(v) => v,
            None => crate::rt::panic_str("overflow in `Step::forward`", file!(), line!(), column!()),
        }
    }

    /// Like `backward_checked`, but panics on overflow.
    fn backward(start: Self, count: usize) -> Self {
        match Step::backward_checked(start, count) {
            Some(v) => v,
            None => crate::rt::panic_str("overflow in `Step::backward`", file!(), line!(), column!()),
        }
    }

    /// Like `forward`, without overflow checks.
    unsafe fn forward_unchecked(start: Self, count: usize) -> Self {
        Step::forward(start, count)
    }

    /// Like `backward`, without overflow checks.
    unsafe fn backward_unchecked(start: Self, count: usize) -> Self {
        Step::backward(start, count)
    }
}

// Integers no wider than 64 bits: exact arithmetic in i128.
macro_rules! __std_step_narrow {
    ($($t:ty)*) => {
        $(
            impl Step for $t {
                #[inline]
                fn steps_between(start: &$t, end: &$t) -> (usize, Option<usize>) {
                    if *start <= *end {
                        let d = (*end as i128) - (*start as i128);
                        if d > (usize::MAX as i128) {
                            (usize::MAX, None)
                        } else {
                            (d as usize, Some(d as usize))
                        }
                    } else {
                        (0, None)
                    }
                }
                #[inline]
                fn forward_checked(start: $t, n: usize) -> Option<$t> {
                    let r = (start as i128) + (n as i128);
                    if r > (<$t>::MAX as i128) {
                        None
                    } else {
                        Some(r as $t)
                    }
                }
                #[inline]
                fn backward_checked(start: $t, n: usize) -> Option<$t> {
                    let r = (start as i128) - (n as i128);
                    if r < (<$t>::MIN as i128) {
                        None
                    } else {
                        Some(r as $t)
                    }
                }
                #[inline]
                unsafe fn forward_unchecked(start: $t, n: usize) -> $t {
                    start.wrapping_add(n as $t)
                }
                #[inline]
                unsafe fn backward_unchecked(start: $t, n: usize) -> $t {
                    start.wrapping_sub(n as $t)
                }
            }

            impl ExactSizeIterator for Range<$t> {}
            impl ExactSizeIterator for RangeInclusive<$t> {}
            unsafe impl TrustedLen for Range<$t> {}
        )*
    };
}

__std_step_narrow! { i8 i16 i32 i64 isize u8 u16 u32 u64 usize }

impl Step for u128 {
    #[inline]
    fn steps_between(start: &u128, end: &u128) -> (usize, Option<usize>) {
        if *start <= *end {
            let d = *end - *start;
            if d > (usize::MAX as u128) {
                (usize::MAX, None)
            } else {
                (d as usize, Some(d as usize))
            }
        } else {
            (0, None)
        }
    }
    #[inline]
    fn forward_checked(start: u128, n: usize) -> Option<u128> {
        start.checked_add(n as u128)
    }
    #[inline]
    fn backward_checked(start: u128, n: usize) -> Option<u128> {
        start.checked_sub(n as u128)
    }
    #[inline]
    unsafe fn forward_unchecked(start: u128, n: usize) -> u128 {
        start.wrapping_add(n as u128)
    }
    #[inline]
    unsafe fn backward_unchecked(start: u128, n: usize) -> u128 {
        start.wrapping_sub(n as u128)
    }
}

impl Step for i128 {
    #[inline]
    fn steps_between(start: &i128, end: &i128) -> (usize, Option<usize>) {
        if *start <= *end {
            let d = (*end as u128).wrapping_sub(*start as u128);
            if d > (usize::MAX as u128) {
                (usize::MAX, None)
            } else {
                (d as usize, Some(d as usize))
            }
        } else {
            (0, None)
        }
    }
    #[inline]
    fn forward_checked(start: i128, n: usize) -> Option<i128> {
        start.checked_add(n as i128)
    }
    #[inline]
    fn backward_checked(start: i128, n: usize) -> Option<i128> {
        start.checked_sub(n as i128)
    }
    #[inline]
    unsafe fn forward_unchecked(start: i128, n: usize) -> i128 {
        start.wrapping_add(n as i128)
    }
    #[inline]
    unsafe fn backward_unchecked(start: i128, n: usize) -> i128 {
        start.wrapping_sub(n as i128)
    }
}

impl ExactSizeIterator for Range<u128> {}
impl ExactSizeIterator for Range<i128> {}

impl Step for char {
    #[inline]
    fn steps_between(start: &char, end: &char) -> (usize, Option<usize>) {
        let start = *start as u32;
        let end = *end as u32;
        if start <= end {
            let mut count = (end - start) as usize;
            if start < 0xD800 && 0xE000 <= end {
                count -= 0x800;
            }
            (count, Some(count))
        } else {
            (0, None)
        }
    }
    #[inline]
    fn forward_checked(start: char, count: usize) -> Option<char> {
        let start = start as u32;
        let mut res = match Step::forward_checked(start, count) {
            Some(v) => v,
            None => return None,
        };
        if start < 0xD800 && 0xD800 <= res {
            res = match Step::forward_checked(res, 0x800) {
                Some(v) => v,
                None => return None,
            };
        }
        if res <= 0x10FFFF {
            Some(unsafe { crate::char::from_u32_unchecked(res) })
        } else {
            None
        }
    }
    #[inline]
    fn backward_checked(start: char, count: usize) -> Option<char> {
        let start = start as u32;
        let mut res = match Step::backward_checked(start, count) {
            Some(v) => v,
            None => return None,
        };
        if start >= 0xE000 && 0xE000 > res {
            res = match Step::backward_checked(res, 0x800) {
                Some(v) => v,
                None => return None,
            };
        }
        Some(unsafe { crate::char::from_u32_unchecked(res) })
    }
}

// ---------------------------------------------------------------------------
// Range<A>

impl<A: Step> Iterator for Range<A> {
    type Item = A;

    #[inline]
    fn next(&mut self) -> Option<A> {
        if self.start < self.end {
            let n = unsafe { Step::forward_unchecked(self.start.clone(), 1) };
            Some(mem::replace(&mut self.start, n))
        } else {
            None
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.start < self.end {
            Step::steps_between(&self.start, &self.end)
        } else {
            (0, Some(0))
        }
    }

    #[inline]
    fn count(self) -> usize {
        if self.start < self.end {
            match Step::steps_between(&self.start, &self.end).1 {
                Some(n) => n,
                None => crate::rt::panic_str("count overflowed usize", file!(), line!(), column!()),
            }
        } else {
            0
        }
    }

    #[inline]
    fn nth(&mut self, n: usize) -> Option<A> {
        if let Some(plus_n) = Step::forward_checked(self.start.clone(), n) {
            if plus_n < self.end {
                self.start = unsafe { Step::forward_unchecked(plus_n.clone(), 1) };
                return Some(plus_n);
            }
        }
        self.start = self.end.clone();
        None
    }

    #[inline]
    fn last(self) -> Option<A> {
        let mut s = self;
        s.next_back()
    }

    #[inline]
    fn min(self) -> Option<A>
    where
        A: Ord,
    {
        let mut s = self;
        s.next()
    }

    #[inline]
    fn max(self) -> Option<A>
    where
        A: Ord,
    {
        let mut s = self;
        s.next_back()
    }
}

impl<A: Step> DoubleEndedIterator for Range<A> {
    #[inline]
    fn next_back(&mut self) -> Option<A> {
        if self.start < self.end {
            self.end = unsafe { Step::backward_unchecked(self.end.clone(), 1) };
            Some(self.end.clone())
        } else {
            None
        }
    }

    #[inline]
    fn nth_back(&mut self, n: usize) -> Option<A> {
        if let Some(minus_n) = Step::backward_checked(self.end.clone(), n) {
            if minus_n > self.start {
                self.end = unsafe { Step::backward_unchecked(minus_n, 1) };
                return Some(self.end.clone());
            }
        }
        self.end = self.start.clone();
        None
    }
}

impl<A: Step> FusedIterator for Range<A> {}

// ---------------------------------------------------------------------------
// RangeFrom<A>

impl<A: Step> Iterator for RangeFrom<A> {
    type Item = A;

    #[inline]
    fn next(&mut self) -> Option<A> {
        let n = Step::forward(self.start.clone(), 1);
        Some(mem::replace(&mut self.start, n))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (usize::MAX, None)
    }

    #[inline]
    fn nth(&mut self, n: usize) -> Option<A> {
        let plus_n = Step::forward(self.start.clone(), n);
        self.start = Step::forward(plus_n.clone(), 1);
        Some(plus_n)
    }
}

impl<A: Step> FusedIterator for RangeFrom<A> {}

// ---------------------------------------------------------------------------
// RangeInclusive<A>

impl<A: Step> Iterator for RangeInclusive<A> {
    type Item = A;

    #[inline]
    fn next(&mut self) -> Option<A> {
        if self.exhausted || !(self.start <= self.end) {
            return None;
        }
        let is_iterating = self.start < self.end;
        Some(if is_iterating {
            let n = unsafe { Step::forward_unchecked(self.start.clone(), 1) };
            mem::replace(&mut self.start, n)
        } else {
            self.exhausted = true;
            self.start.clone()
        })
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.exhausted || !(self.start <= self.end) {
            return (0, Some(0));
        }
        let hint = Step::steps_between(&self.start, &self.end);
        let lo = hint.0.saturating_add(1);
        let hi = match hint.1 {
            Some(steps) => steps.checked_add(1),
            None => None,
        };
        (lo, hi)
    }

    #[inline]
    fn count(self) -> usize {
        if self.exhausted || !(self.start <= self.end) {
            return 0;
        }
        match Step::steps_between(&self.start, &self.end).1 {
            Some(steps) => match steps.checked_add(1) {
                Some(n) => n,
                None => crate::rt::panic_str("count overflowed usize", file!(), line!(), column!()),
            },
            None => crate::rt::panic_str("count overflowed usize", file!(), line!(), column!()),
        }
    }

    #[inline]
    fn nth(&mut self, n: usize) -> Option<A> {
        if self.exhausted || !(self.start <= self.end) {
            return None;
        }
        if let Some(plus_n) = Step::forward_checked(self.start.clone(), n) {
            if plus_n < self.end {
                self.start = unsafe { Step::forward_unchecked(plus_n.clone(), 1) };
                return Some(plus_n);
            } else if plus_n == self.end {
                self.start = plus_n.clone();
                self.exhausted = true;
                return Some(plus_n);
            }
        }
        self.start = self.end.clone();
        self.exhausted = true;
        None
    }

    #[inline]
    fn last(self) -> Option<A> {
        let mut s = self;
        s.next_back()
    }

    #[inline]
    fn min(self) -> Option<A>
    where
        A: Ord,
    {
        let mut s = self;
        s.next()
    }

    #[inline]
    fn max(self) -> Option<A>
    where
        A: Ord,
    {
        let mut s = self;
        s.next_back()
    }
}

impl<A: Step> DoubleEndedIterator for RangeInclusive<A> {
    #[inline]
    fn next_back(&mut self) -> Option<A> {
        if self.exhausted || !(self.start <= self.end) {
            return None;
        }
        let is_iterating = self.start < self.end;
        Some(if is_iterating {
            let n = unsafe { Step::backward_unchecked(self.end.clone(), 1) };
            mem::replace(&mut self.end, n)
        } else {
            self.exhausted = true;
            self.end.clone()
        })
    }

    #[inline]
    fn nth_back(&mut self, n: usize) -> Option<A> {
        if self.exhausted || !(self.start <= self.end) {
            return None;
        }
        if let Some(minus_n) = Step::backward_checked(self.end.clone(), n) {
            if minus_n > self.start {
                self.end = unsafe { Step::backward_unchecked(minus_n.clone(), 1) };
                return Some(minus_n);
            } else if minus_n == self.start {
                self.end = minus_n.clone();
                self.exhausted = true;
                return Some(minus_n);
            }
        }
        self.end = self.start.clone();
        self.exhausted = true;
        None
    }
}

impl<A: Step> FusedIterator for RangeInclusive<A> {}
