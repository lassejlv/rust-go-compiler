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

impl Pattern for char {
    #[inline]
    fn find_in(&mut self, haystack: &str, start: usize, end: usize) -> Option<(usize, usize)> {
        let c = *self as u32;
        if c < 0x80 {
            let b = c as u8;
            let bytes = haystack.as_bytes();
            let mut i = start;
            while i < end {
                if bytes[i] == b {
                    return Some((i, i + 1));
                }
                i += 1;
            }
            None
        } else {
            pred_find(self, haystack, start, end)
        }
    }
    #[inline]
    fn rfind_in(&mut self, haystack: &str, start: usize, end: usize) -> Option<(usize, usize)> {
        let c = *self as u32;
        if c < 0x80 {
            let b = c as u8;
            let bytes = haystack.as_bytes();
            let mut i = end;
            while i > start {
                i -= 1;
                if bytes[i] == b {
                    return Some((i, i + 1));
                }
            }
            None
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
    let h = haystack.as_bytes();
    let nb = needle.as_bytes();
    let first = nb[0];
    let last_start = end - n;
    let mut i = start;
    while i <= last_start {
        if h[i] == first && bytes_eq_at(h, i, nb) {
            return Some((i, i + n));
        }
        i += 1;
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
    let mut i = end - n + 1;
    while i > start {
        i -= 1;
        if bytes_eq_at(h, i, nb) {
            return Some((i, i + n));
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
