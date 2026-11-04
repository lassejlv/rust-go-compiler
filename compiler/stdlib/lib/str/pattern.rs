//! The string `Pattern` API.
//!
//! A pattern is searched for inside a window `haystack[start..end]` of a
//! string; all positions are absolute byte offsets into `haystack`.

use super::{decode_before, decode_at};

/// A string pattern: `char`, `&str`, `&String`, `&&str`, `[char; N]`,
/// `&[char; N]`, `&[char]` or a `FnMut(char) -> bool` predicate.
pub trait Pattern: Sized {
    /// Finds the first match inside `haystack[start..end]`.
    fn find_in(&mut self, haystack: &str, start: usize, end: usize) -> Option<(usize, usize)>;

    /// Finds the last match inside `haystack[start..end]`.
    fn rfind_in(&mut self, haystack: &str, start: usize, end: usize) -> Option<(usize, usize)>;

    /// If the pattern matches at the very start of `haystack[start..end]`, returns the end of that match.
    fn prefix_match(&mut self, haystack: &str, start: usize, end: usize) -> Option<usize>;

    /// If the pattern matches at the very end of `haystack[start..end]`, returns the start of that match.
    fn suffix_match(&mut self, haystack: &str, start: usize, end: usize) -> Option<usize>;

    /// Checks whether the pattern matches anywhere in the haystack.
    #[inline]
    fn is_contained_in(mut self, haystack: &str) -> bool {
        self.find_in(haystack, 0, haystack.len()).is_some()
    }

    /// Checks whether the pattern matches at the front of the haystack.
    #[inline]
    fn is_prefix_of(mut self, haystack: &str) -> bool {
        self.prefix_match(haystack, 0, haystack.len()).is_some()
    }

    /// Checks whether the pattern matches at the back of the haystack.
    #[inline]
    fn is_suffix_of(mut self, haystack: &str) -> bool {
        self.suffix_match(haystack, 0, haystack.len()).is_some()
    }

    /// Removes the pattern from the front of haystack, if it matches.
    #[inline]
    fn strip_prefix_of(mut self, haystack: &str) -> Option<&str> {
        match self.prefix_match(haystack, 0, haystack.len()) {
            Some(e) => Some(unsafe { haystack.get_unchecked(e..haystack.len()) }),
            None => None,
        }
    }

    /// Removes the pattern from the back of haystack, if it matches.
    #[inline]
    fn strip_suffix_of(mut self, haystack: &str) -> Option<&str> {
        match self.suffix_match(haystack, 0, haystack.len()) {
            Some(s) => Some(unsafe { haystack.get_unchecked(0..s) }),
            None => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Character predicates

/// A pattern matching single characters.
pub(crate) trait CharPredicate {
    fn matches_char(&mut self, c: char) -> bool;
}

#[inline]
fn pred_find<P: CharPredicate>(p: &mut P, h: &str, start: usize, end: usize) -> Option<(usize, usize)> {
    let mut i = start;
    while i < end {
        let (c, len) = decode_at(h, i);
        if p.matches_char(c) {
            return Some((i, i + len));
        }
        i += len;
    }
    None
}

#[inline]
fn pred_rfind<P: CharPredicate>(p: &mut P, h: &str, start: usize, end: usize) -> Option<(usize, usize)> {
    let mut i = end;
    while i > start {
        let (c, len) = decode_before(h, i);
        if p.matches_char(c) {
            return Some((i - len, i));
        }
        i -= len;
    }
    None
}

#[inline]
fn pred_prefix<P: CharPredicate>(p: &mut P, h: &str, start: usize, end: usize) -> Option<usize> {
    if start < end {
        let (c, len) = decode_at(h, start);
        if p.matches_char(c) {
            return Some(start + len);
        }
    }
    None
}

#[inline]
fn pred_suffix<P: CharPredicate>(p: &mut P, h: &str, start: usize, end: usize) -> Option<usize> {
    if start < end {
        let (c, len) = decode_before(h, end);
        if p.matches_char(c) {
            return Some(end - len);
        }
    }
    None
}

macro_rules! __std_pattern_via_pred {
    () => {
        #[inline]
        fn find_in(&mut self, haystack: &str, start: usize, end: usize) -> Option<(usize, usize)> {
            pred_find(self, haystack, start, end)
        }
        #[inline]
        fn rfind_in(&mut self, haystack: &str, start: usize, end: usize) -> Option<(usize, usize)> {
            pred_rfind(self, haystack, start, end)
        }
        #[inline]
        fn prefix_match(&mut self, haystack: &str, start: usize, end: usize) -> Option<usize> {
            pred_prefix(self, haystack, start, end)
        }
        #[inline]
        fn suffix_match(&mut self, haystack: &str, start: usize, end: usize) -> Option<usize> {
            pred_suffix(self, haystack, start, end)
        }
    };
}

impl CharPredicate for char {
    #[inline]
    fn matches_char(&mut self, c: char) -> bool {
        *self == c
    }
}

/// Patterns that can be searched from both ends with the same results
/// (std's `DoubleEndedSearcher`): the split/match iterators are
/// double-ended only for these (char-like patterns, not string ones).
#[doc(hidden)]
pub trait DoubleEndedPattern: Pattern {}
impl DoubleEndedPattern for char {}
impl<F: FnMut(char) -> bool> DoubleEndedPattern for F {}
impl<const N: usize> DoubleEndedPattern for [char; N] {}
impl<'b, const N: usize> DoubleEndedPattern for &'b [char; N] {}
impl<'b> DoubleEndedPattern for &'b [char] {}

impl Pattern for char {
    #[inline]
    fn find_in(&mut self, haystack: &str, start: usize, end: usize) -> Option<(usize, usize)> {
        let c = *self as u32;
        if start >= end {
            return None;
        }
        if c < 0x80 {
            let p = unsafe { crate::intrinsics::ptr_add(haystack.as_ptr(), start) };
            let i = unsafe { crate::intrinsics::memchr(p, c as u8, end - start) };
            if i < end - start {
                Some((start + i, start + i + 1))
            } else {
                None
            }
        } else {
            let mut buf = [0u8; 4];
            let s = self.encode_utf8(&mut buf);
            substr_find(haystack, s, start, end)
        }
    }
    #[inline]
    fn rfind_in(&mut self, haystack: &str, start: usize, end: usize) -> Option<(usize, usize)> {
        let c = *self as u32;
        if start >= end {
            return None;
        }
        if c < 0x80 {
            let p = unsafe { crate::intrinsics::ptr_add(haystack.as_ptr(), start) };
            let i = unsafe { crate::intrinsics::memrchr(p, c as u8, end - start) };
            if i < end - start {
                Some((start + i, start + i + 1))
            } else {
                None
            }
        } else {
            pred_rfind(self, haystack, start, end)
        }
    }
    #[inline]
    fn prefix_match(&mut self, haystack: &str, start: usize, end: usize) -> Option<usize> {
        pred_prefix(self, haystack, start, end)
    }
    #[inline]
    fn suffix_match(&mut self, haystack: &str, start: usize, end: usize) -> Option<usize> {
        pred_suffix(self, haystack, start, end)
    }
}

impl<F: FnMut(char) -> bool> CharPredicate for F {
    #[inline]
    fn matches_char(&mut self, c: char) -> bool {
        (*self)(c)
    }
}

impl<F: FnMut(char) -> bool> Pattern for F {
    __std_pattern_via_pred! {}
}

impl<const N: usize> CharPredicate for [char; N] {
    #[inline]
    fn matches_char(&mut self, c: char) -> bool {
        let mut i = 0;
        while i < N {
            if self[i] == c {
                return true;
            }
            i += 1;
        }
        false
    }
}

impl<const N: usize> Pattern for [char; N] {
    __std_pattern_via_pred! {}
}

impl<'b, const N: usize> CharPredicate for &'b [char; N] {
    #[inline]
    fn matches_char(&mut self, c: char) -> bool {
        let mut i = 0;
        while i < N {
            if self[i] == c {
                return true;
            }
            i += 1;
        }
        false
    }
}

impl<'b, const N: usize> Pattern for &'b [char; N] {
    __std_pattern_via_pred! {}
}

impl<'b> CharPredicate for &'b [char] {
    #[inline]
    fn matches_char(&mut self, c: char) -> bool {
        let n = self.len();
        let mut i = 0;
        while i < n {
            if self[i] == c {
                return true;
            }
            i += 1;
        }
        false
    }
}

impl<'b> Pattern for &'b [char] {
    __std_pattern_via_pred! {}
}

// ---------------------------------------------------------------------------
// Substring patterns

#[inline]
fn bytes_eq_at(h: &[u8], at: usize, n: &[u8]) -> bool {
    let nl = n.len();
    let mut j = 0;
    while j < nl {
        if h[at + j] != n[j] {
            return false;
        }
        j += 1;
    }
    true
}

pub(crate) fn substr_find(haystack: &str, needle: &str, start: usize, end: usize) -> Option<(usize, usize)> {
    let n = needle.len();
    if start > end {
        return None;
    }
    if n == 0 {
        return Some((start, start));
    }
    if end - start < n {
        return None;
    }
    use crate::intrinsics::{compare_bytes, memchr, memmem, ptr_add};
    let h = haystack.as_ptr();
    let nb = needle.as_ptr();
    unsafe {
        let first = *nb;
        let last_start = end - n;
        // candidates by the first byte (memchr) while comparing them stays
        // cheap (at most n bytes each); then the C library's memmem (linear
        // in the worst case, but slower to start)
        let mut i = start;
        let mut budget = (64 + 2 * (end - start)) / n;
        while i <= last_start {
            let k = memchr(ptr_add(h, i), first, last_start + 1 - i);
            if k > last_start - i {
                return None;
            }
            i += k;
            if compare_bytes(ptr_add(h, i + 1), ptr_add(nb, 1), n - 1) == 0 {
                return Some((i, i + n));
            }
            i += 1;
            if budget == 0 {
                let r = memmem(ptr_add(h, i), end - i, nb, n);
                return if r == usize::MAX { None } else { Some((i + r, i + r + n)) };
            }
            budget -= 1;
        }
        None
    }
}

/// KMP failure function of `nb` read forwards (`rev`: backwards).
fn kmp_table(nb: &[u8], rev: bool) -> Vec<usize> {
    let n = nb.len();
    let at = |i: usize| if rev { nb[n - 1 - i] } else { nb[i] };
    let mut fail = vec![0usize; n];
    let mut k = 0;
    let mut i = 1;
    while i < n {
        while k > 0 && at(i) != at(k) {
            k = fail[k - 1];
        }
        if at(i) == at(k) {
            k += 1;
        }
        fail[i] = k;
        i += 1;
    }
    fail
}

fn kmp_rfind(h: &[u8], nb: &[u8], start: usize, end: usize) -> Option<(usize, usize)> {
    let n = nb.len();
    let fail = kmp_table(nb, true);
    let mut k = 0;
    let mut i = end;
    while i > start {
        i -= 1;
        while k > 0 && h[i] != nb[n - 1 - k] {
            k = fail[k - 1];
        }
        if h[i] == nb[n - 1 - k] {
            k += 1;
        }
        if k == n {
            return Some((i, i + n));
        }
    }
    None
}

pub(crate) fn substr_rfind(haystack: &str, needle: &str, start: usize, end: usize) -> Option<(usize, usize)> {
    let n = needle.len();
    if start > end {
        return None;
    }
    if n == 0 {
        return Some((end, end));
    }
    if end - start < n {
        return None;
    }
    let h = haystack.as_bytes();
    let nb = needle.as_bytes();
    let last = nb[n - 1];
    let mut budget = 64 + 4 * (end - start);
    let mut i = end - n + 1;
    while i > start {
        i -= 1;
        if h[i + n - 1] == last {
            // compare backwards from the end (see substr_find)
            let mut j = 1;
            while j < n && h[i + n - 1 - j] == nb[n - 1 - j] {
                j += 1;
            }
            if j == n {
                return Some((i, i + n));
            }
            if j > budget {
                return kmp_rfind(h, nb, start, i + n - 1);
            }
            budget -= j;
        }
    }
    None
}

#[inline]
fn substr_prefix(haystack: &str, needle: &str, start: usize, end: usize) -> Option<usize> {
    let n = needle.len();
    if start <= end && end - start >= n && bytes_eq_at(haystack.as_bytes(), start, needle.as_bytes()) {
        Some(start + n)
    } else {
        None
    }
}

#[inline]
fn substr_suffix(haystack: &str, needle: &str, start: usize, end: usize) -> Option<usize> {
    let n = needle.len();
    if start <= end && end - start >= n && bytes_eq_at(haystack.as_bytes(), end - n, needle.as_bytes()) {
        Some(end - n)
    } else {
        None
    }
}

macro_rules! __std_pattern_via_str {
    ($t:ty, $s:ident => $e:expr) => {
        impl<'b, 'c> Pattern for $t {
            #[inline]
            fn find_in(&mut self, haystack: &str, start: usize, end: usize) -> Option<(usize, usize)> {
                let $s = &*self;
                substr_find(haystack, $e, start, end)
            }
            #[inline]
            fn rfind_in(&mut self, haystack: &str, start: usize, end: usize) -> Option<(usize, usize)> {
                let $s = &*self;
                substr_rfind(haystack, $e, start, end)
            }
            #[inline]
            fn prefix_match(&mut self, haystack: &str, start: usize, end: usize) -> Option<usize> {
                let $s = &*self;
                substr_prefix(haystack, $e, start, end)
            }
            #[inline]
            fn suffix_match(&mut self, haystack: &str, start: usize, end: usize) -> Option<usize> {
                let $s = &*self;
                substr_suffix(haystack, $e, start, end)
            }
        }
    };
}

__std_pattern_via_str! { &'b str, s => *s }
__std_pattern_via_str! { &'b String, s => s.as_str() }
__std_pattern_via_str! { &'c &'b str, s => **s }

/// The searcher state shared by the split/match iterators: the window
/// `[pos, back]` that remains to be searched.
#[derive(Clone, Debug)]
pub(crate) struct SearchWindow {
    pub(crate) pos: usize,
    pub(crate) back: usize,
    pub(crate) fw_done: bool,
    pub(crate) bw_done: bool,
}

impl SearchWindow {
    #[inline]
    pub(crate) fn new(len: usize) -> SearchWindow {
        SearchWindow { pos: 0, back: len, fw_done: false, bw_done: false }
    }

    #[inline]
    pub(crate) fn next_match<P: Pattern>(&mut self, pat: &mut P, haystack: &str) -> Option<(usize, usize)> {
        if self.fw_done || self.bw_done || self.pos > self.back {
            return None;
        }
        match pat.find_in(haystack, self.pos, self.back) {
            Some((a, b)) => {
                if a == b {
                    // Empty match: step over one character.
                    if b >= self.back {
                        self.fw_done = true;
                    } else {
                        let (_, len) = decode_at(haystack, b);
                        self.pos = b + len;
                    }
                } else {
                    self.pos = b;
                }
                Some((a, b))
            }
            None => {
                self.fw_done = true;
                None
            }
        }
    }

    #[inline]
    pub(crate) fn next_match_back<P: Pattern>(&mut self, pat: &mut P, haystack: &str) -> Option<(usize, usize)> {
        if self.fw_done || self.bw_done || self.pos > self.back {
            return None;
        }
        match pat.rfind_in(haystack, self.pos, self.back) {
            Some((a, b)) => {
                if a == b {
                    if a <= self.pos {
                        self.bw_done = true;
                    } else {
                        let (_, len) = decode_before(haystack, a);
                        self.back = a - len;
                    }
                } else {
                    self.back = a;
                }
                Some((a, b))
            }
            None => {
                self.bw_done = true;
                None
            }
        }
    }
}
