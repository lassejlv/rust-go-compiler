//! Iterators over string slices.

use super::pattern::{Pattern, SearchWindow};
use super::{decode_at, decode_before};
use crate::fmt;
use crate::iter::{Copied, DoubleEndedIterator, ExactSizeIterator, FusedIterator, Iterator};
use crate::slice;

// ---------------------------------------------------------------------------
// Chars / CharIndices / Bytes

/// An iterator over the `char`s of a string slice.
#[derive(Clone)]
#[must_use]
pub struct Chars<'a> {
    s: &'a str,
    front: usize,
    back: usize,
}

impl<'a> Chars<'a> {
    #[inline]
    pub(crate) fn new(s: &'a str) -> Chars<'a> {
        Chars { s, front: 0, back: s.len() }
    }

    /// Views the underlying data as a subslice of the original data.
    #[inline]
    pub fn as_str(&self) -> &'a str {
        unsafe { self.s.get_unchecked(self.front..self.back) }
    }
}

impl<'a> Iterator for Chars<'a> {
    type Item = char;
    #[inline]
    fn next(&mut self) -> Option<char> {
        if self.front < self.back {
            let (c, len) = decode_at(self.s, self.front);
            self.front += len;
            Some(c)
        } else {
            None
        }
    }
    #[inline]
    fn count(self) -> usize {
        let b = self.s.as_bytes();
        let mut n = 0;
        let mut i = self.front;
        while i < self.back {
            if (b[i] & 0xC0) != 0x80 {
                n += 1;
            }
            i += 1;
        }
        n
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.back - self.front;
        ((len + 3) / 4, Some(len))
    }
    #[inline]
    fn last(self) -> Option<char> {
        let mut s = self;
        s.next_back()
    }
}

impl<'a> DoubleEndedIterator for Chars<'a> {
    #[inline]
    fn next_back(&mut self) -> Option<char> {
        if self.front < self.back {
            let (c, len) = decode_before(self.s, self.back);
            self.back -= len;
            Some(c)
        } else {
            None
        }
    }
}

impl<'a> FusedIterator for Chars<'a> {}

impl<'a> fmt::Debug for Chars<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Chars(")?;
        f.debug_list().entries(self.clone()).finish()?;
        f.write_str(")")
    }
}

/// An iterator over the `char`s of a string slice, and their positions.
#[derive(Clone)]
#[must_use]
pub struct CharIndices<'a> {
    iter: Chars<'a>,
}

impl<'a> fmt::Debug for CharIndices<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CharIndices").field("front_offset", &self.iter.front).field("iter", &self.iter).finish()
    }
}

impl<'a> CharIndices<'a> {
    #[inline]
    pub(crate) fn new(s: &'a str) -> CharIndices<'a> {
        CharIndices { iter: Chars::new(s) }
    }

    /// Views the underlying data as a subslice of the original data.
    #[inline]
    pub fn as_str(&self) -> &'a str {
        self.iter.as_str()
    }

    /// Returns the byte position of the next character, or the length of the underlying string if there are no more characters.
    #[inline]
    pub fn offset(&self) -> usize {
        self.iter.front
    }
}

impl<'a> Iterator for CharIndices<'a> {
    type Item = (usize, char);
    #[inline]
    fn next(&mut self) -> Option<(usize, char)> {
        let i = self.iter.front;
        match self.iter.next() {
            Some(c) => Some((i, c)),
            None => None,
        }
    }
    #[inline]
    fn count(self) -> usize {
        self.iter.count()
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
    #[inline]
    fn last(self) -> Option<(usize, char)> {
        let mut s = self;
        s.next_back()
    }
}

impl<'a> DoubleEndedIterator for CharIndices<'a> {
    #[inline]
    fn next_back(&mut self) -> Option<(usize, char)> {
        match self.iter.next_back() {
            Some(c) => Some((self.iter.back, c)),
            None => None,
        }
    }
}

impl<'a> FusedIterator for CharIndices<'a> {}

/// An iterator over the bytes of a string slice.
#[derive(Clone, Debug)]
#[must_use]
pub struct Bytes<'a>(Copied<slice::Iter<'a, u8>>);

impl<'a> Bytes<'a> {
    #[inline]
    pub(crate) fn new(b: &'a [u8]) -> Bytes<'a> {
        Bytes(b.iter().copied())
    }
}

impl<'a> Iterator for Bytes<'a> {
    type Item = u8;
    #[inline]
    fn next(&mut self) -> Option<u8> {
        self.0.next()
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
    #[inline]
    fn count(self) -> usize {
        self.0.len()
    }
    #[inline]
    fn nth(&mut self, n: usize) -> Option<u8> {
        self.0.nth(n)
    }
    #[inline]
    fn last(self) -> Option<u8> {
        let mut s = self;
        s.0.next_back()
    }
}

impl<'a> DoubleEndedIterator for Bytes<'a> {
    #[inline]
    fn next_back(&mut self) -> Option<u8> {
        self.0.next_back()
    }
}

impl<'a> ExactSizeIterator for Bytes<'a> {
    #[inline]
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl<'a> FusedIterator for Bytes<'a> {}

// ---------------------------------------------------------------------------
// Split and friends

/// An iterator over substrings separated by a pattern.
#[must_use]
pub struct Split<'a, P: Pattern> {
    haystack: &'a str,
    start: usize,
    end: usize,
    window: SearchWindow,
    pat: P,
    allow_trailing_empty: bool,
    finished: bool,
}

impl<'a, P: Pattern + Clone> Clone for Split<'a, P> {
    fn clone(&self) -> Self {
        Split {
            haystack: self.haystack,
            start: self.start,
            end: self.end,
            window: self.window.clone(),
            pat: self.pat.clone(),
            allow_trailing_empty: self.allow_trailing_empty,
            finished: self.finished,
        }
    }
}

impl<'a, P: Pattern> fmt::Debug for Split<'a, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Split")
            .field("start", &self.start)
            .field("end", &self.end)
            .field("allow_trailing_empty", &self.allow_trailing_empty)
            .field("finished", &self.finished)
            .finish()
    }
}

impl<'a, P: Pattern> Split<'a, P> {
    #[inline]
    pub(crate) fn new(haystack: &'a str, pat: P, allow_trailing_empty: bool) -> Split<'a, P> {
        Split {
            haystack,
            start: 0,
            end: haystack.len(),
            window: SearchWindow::new(haystack.len()),
            pat,
            allow_trailing_empty,
            finished: false,
        }
    }

    #[inline]
    fn get_end(&mut self) -> Option<&'a str> {
        if !self.finished {
            self.finished = true;
            if self.allow_trailing_empty || self.end > self.start {
                return Some(unsafe { self.haystack.get_unchecked(self.start..self.end) });
            }
        }
        None
    }

    #[inline]
    fn next_fwd(&mut self) -> Option<&'a str> {
        if self.finished {
            return None;
        }
        match self.window.next_match(&mut self.pat, self.haystack) {
            Some((a, b)) => {
                let elt = unsafe { self.haystack.get_unchecked(self.start..a) };
                self.start = b;
                Some(elt)
            }
            None => self.get_end(),
        }
    }

    #[inline]
    fn next_inclusive(&mut self) -> Option<&'a str> {
        if self.finished {
            return None;
        }
        match self.window.next_match(&mut self.pat, self.haystack) {
            Some((_, b)) => {
                let elt = unsafe { self.haystack.get_unchecked(self.start..b) };
                self.start = b;
                Some(elt)
            }
            None => self.get_end(),
        }
    }

    fn next_bwd(&mut self) -> Option<&'a str> {
        if self.finished {
            return None;
        }
        if !self.allow_trailing_empty {
            self.allow_trailing_empty = true;
            match self.next_bwd() {
                Some(elt) if !elt.is_empty() => return Some(elt),
                _ => {
                    if self.finished {
                        return None;
                    }
                }
            }
        }
        match self.window.next_match_back(&mut self.pat, self.haystack) {
            Some((a, b)) => {
                let elt = unsafe { self.haystack.get_unchecked(b..self.end) };
                self.end = a;
                Some(elt)
            }
            None => {
                self.finished = true;
                Some(unsafe { self.haystack.get_unchecked(self.start..self.end) })
            }
        }
    }

    fn next_bwd_inclusive(&mut self) -> Option<&'a str> {
        if self.finished {
            return None;
        }
        if !self.allow_trailing_empty {
            self.allow_trailing_empty = true;
            match self.next_bwd_inclusive() {
                Some(elt) if !elt.is_empty() => return Some(elt),
                _ => {
                    if self.finished {
                        return None;
                    }
                }
            }
        }
        match self.window.next_match_back(&mut self.pat, self.haystack) {
            Some((_, b)) => {
                let elt = unsafe { self.haystack.get_unchecked(b..self.end) };
                self.end = b;
                Some(elt)
            }
            None => {
                self.finished = true;
                Some(unsafe { self.haystack.get_unchecked(self.start..self.end) })
            }
        }
    }

    /// Returns remainder of the split string.
    #[inline]
    pub fn remainder(&self) -> Option<&'a str> {
        if self.finished {
            None
        } else {
            Some(unsafe { self.haystack.get_unchecked(self.start..self.end) })
        }
    }
}

impl<'a, P: Pattern> Iterator for Split<'a, P> {
    type Item = &'a str;
    #[inline]
    fn next(&mut self) -> Option<&'a str> {
        self.next_fwd()
    }
}

impl<'a, P: super::pattern::DoubleEndedPattern> DoubleEndedIterator for Split<'a, P> {
    #[inline]
    fn next_back(&mut self) -> Option<&'a str> {
        self.next_bwd()
    }
}

impl<'a, P: Pattern> FusedIterator for Split<'a, P> {}

/// An iterator over substrings separated by a pattern, including the terminator.
#[must_use]
pub struct SplitInclusive<'a, P: Pattern>(Split<'a, P>);

impl<'a, P: Pattern> SplitInclusive<'a, P> {
    #[inline]
    pub(crate) fn new(haystack: &'a str, pat: P) -> SplitInclusive<'a, P> {
        SplitInclusive(Split::new(haystack, pat, false))
    }
}

impl<'a, P: Pattern + Clone> Clone for SplitInclusive<'a, P> {
    fn clone(&self) -> Self {
        SplitInclusive(self.0.clone())
    }
}

impl<'a, P: Pattern> Iterator for SplitInclusive<'a, P> {
    type Item = &'a str;
    #[inline]
    fn next(&mut self) -> Option<&'a str> {
        self.0.next_inclusive()
    }
}

impl<'a, P: super::pattern::DoubleEndedPattern> DoubleEndedIterator for SplitInclusive<'a, P> {
    #[inline]
    fn next_back(&mut self) -> Option<&'a str> {
        self.0.next_bwd_inclusive()
    }
}

impl<'a, P: Pattern> FusedIterator for SplitInclusive<'a, P> {}

macro_rules! __std_str_split_wrapper {
    ($name:ident, $doc:expr, fwd: $fwd:ident, bwd: $bwd:ident) => {
        #[doc = $doc]
        #[must_use]
        pub struct $name<'a, P: Pattern>(Split<'a, P>);

        impl<'a, P: Pattern> $name<'a, P> {
            #[inline]
            pub(crate) fn new(inner: Split<'a, P>) -> $name<'a, P> {
                $name(inner)
            }

            /// Returns remainder of the split string.
            #[inline]
            pub fn remainder(&self) -> Option<&'a str> {
                self.0.remainder()
            }
        }

        impl<'a, P: Pattern + Clone> Clone for $name<'a, P> {
            fn clone(&self) -> Self {
                $name(self.0.clone())
            }
        }

        impl<'a, P: Pattern> fmt::Debug for $name<'a, P> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_tuple(stringify!($name)).field(&self.0).finish()
            }
        }

        impl<'a, P: Pattern> Iterator for $name<'a, P> {
            type Item = &'a str;
            #[inline]
            fn next(&mut self) -> Option<&'a str> {
                self.0.$fwd()
            }
        }

        impl<'a, P: super::pattern::DoubleEndedPattern> DoubleEndedIterator for $name<'a, P> {
            #[inline]
            fn next_back(&mut self) -> Option<&'a str> {
                self.0.$bwd()
            }
        }

        impl<'a, P: Pattern> FusedIterator for $name<'a, P> {}
    };
}

__std_str_split_wrapper!(RSplit, "Created with the method `rsplit`.", fwd: next_bwd, bwd: next_fwd);
__std_str_split_wrapper!(SplitTerminator, "Created with the method `split_terminator`.", fwd: next_fwd, bwd: next_bwd);
__std_str_split_wrapper!(RSplitTerminator, "Created with the method `rsplit_terminator`.", fwd: next_bwd, bwd: next_fwd);

/// Created with the method `splitn`.
#[must_use]
pub struct SplitN<'a, P: Pattern> {
    iter: Split<'a, P>,
    count: usize,
}

impl<'a, P: Pattern> SplitN<'a, P> {
    #[inline]
    pub(crate) fn new(iter: Split<'a, P>, count: usize) -> SplitN<'a, P> {
        SplitN { iter, count }
    }

    /// Returns remainder of the split string.
    #[inline]
    pub fn remainder(&self) -> Option<&'a str> {
        self.iter.remainder()
    }
}

impl<'a, P: Pattern + Clone> Clone for SplitN<'a, P> {
    fn clone(&self) -> Self {
        SplitN { iter: self.iter.clone(), count: self.count }
    }
}

impl<'a, P: Pattern> Iterator for SplitN<'a, P> {
    type Item = &'a str;
    #[inline]
    fn next(&mut self) -> Option<&'a str> {
        match self.count {
            0 => None,
            1 => {
                self.count = 0;
                self.iter.get_end()
            }
            _ => {
                self.count -= 1;
                self.iter.next_fwd()
            }
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.count))
    }
}

impl<'a, P: Pattern> FusedIterator for SplitN<'a, P> {}

/// Created with the method `rsplitn`.
#[must_use]
pub struct RSplitN<'a, P: Pattern> {
    iter: Split<'a, P>,
    count: usize,
}

impl<'a, P: Pattern> RSplitN<'a, P> {
    #[inline]
    pub(crate) fn new(iter: Split<'a, P>, count: usize) -> RSplitN<'a, P> {
        RSplitN { iter, count }
    }

    /// Returns remainder of the split string.
    #[inline]
    pub fn remainder(&self) -> Option<&'a str> {
        self.iter.remainder()
    }
}

impl<'a, P: Pattern + Clone> Clone for RSplitN<'a, P> {
    fn clone(&self) -> Self {
        RSplitN { iter: self.iter.clone(), count: self.count }
    }
}

impl<'a, P: Pattern> Iterator for RSplitN<'a, P> {
    type Item = &'a str;
    #[inline]
    fn next(&mut self) -> Option<&'a str> {
        match self.count {
            0 => None,
            1 => {
                self.count = 0;
                self.iter.get_end()
            }
            _ => {
                self.count -= 1;
                self.iter.next_bwd()
            }
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.count))
    }
}

impl<'a, P: Pattern> FusedIterator for RSplitN<'a, P> {}

// ---------------------------------------------------------------------------
// Matches

/// Created with the method `matches`.
#[must_use]
pub struct Matches<'a, P: Pattern> {
    haystack: &'a str,
    window: SearchWindow,
    pat: P,
}

impl<'a, P: Pattern> Matches<'a, P> {
    #[inline]
    pub(crate) fn new(haystack: &'a str, pat: P) -> Matches<'a, P> {
        Matches { haystack, window: SearchWindow::new(haystack.len()), pat }
    }
}

impl<'a, P: Pattern + Clone> Clone for Matches<'a, P> {
    fn clone(&self) -> Self {
        Matches { haystack: self.haystack, window: self.window.clone(), pat: self.pat.clone() }
    }
}

impl<'a, P: Pattern> Iterator for Matches<'a, P> {
    type Item = &'a str;
    #[inline]
    fn next(&mut self) -> Option<&'a str> {
        match self.window.next_match(&mut self.pat, self.haystack) {
            Some((a, b)) => Some(unsafe { self.haystack.get_unchecked(a..b) }),
            None => None,
        }
    }
}

impl<'a, P: Pattern> Matches<'a, P> {
    // (rmatches searches backwards with any pattern)
    #[inline]
    fn match_back(&mut self) -> Option<&'a str> {
        match self.window.next_match_back(&mut self.pat, self.haystack) {
            Some((a, b)) => Some(unsafe { self.haystack.get_unchecked(a..b) }),
            None => None,
        }
    }
}

impl<'a, P: super::pattern::DoubleEndedPattern> DoubleEndedIterator for Matches<'a, P> {
    #[inline]
    fn next_back(&mut self) -> Option<&'a str> {
        self.match_back()
    }
}

impl<'a, P: Pattern> FusedIterator for Matches<'a, P> {}

/// Created with the method `rmatches`.
#[must_use]
pub struct RMatches<'a, P: Pattern>(Matches<'a, P>);

impl<'a, P: Pattern> RMatches<'a, P> {
    #[inline]
    pub(crate) fn new(inner: Matches<'a, P>) -> RMatches<'a, P> {
        RMatches(inner)
    }
}

impl<'a, P: Pattern> Iterator for RMatches<'a, P> {
    type Item = &'a str;
    #[inline]
    fn next(&mut self) -> Option<&'a str> {
        self.0.match_back()
    }
}

impl<'a, P: super::pattern::DoubleEndedPattern> DoubleEndedIterator for RMatches<'a, P> {
    #[inline]
    fn next_back(&mut self) -> Option<&'a str> {
        self.0.next()
    }
}

/// Created with the method `match_indices`.
#[must_use]
pub struct MatchIndices<'a, P: Pattern> {
    haystack: &'a str,
    window: SearchWindow,
    pat: P,
}

impl<'a, P: Pattern> MatchIndices<'a, P> {
    #[inline]
    pub(crate) fn new(haystack: &'a str, pat: P) -> MatchIndices<'a, P> {
        MatchIndices { haystack, window: SearchWindow::new(haystack.len()), pat }
    }
}

impl<'a, P: Pattern + Clone> Clone for MatchIndices<'a, P> {
    fn clone(&self) -> Self {
        MatchIndices { haystack: self.haystack, window: self.window.clone(), pat: self.pat.clone() }
    }
}

impl<'a, P: Pattern> Iterator for MatchIndices<'a, P> {
    type Item = (usize, &'a str);
    #[inline]
    fn next(&mut self) -> Option<(usize, &'a str)> {
        match self.window.next_match(&mut self.pat, self.haystack) {
            Some((a, b)) => Some((a, unsafe { self.haystack.get_unchecked(a..b) })),
            None => None,
        }
    }
}

impl<'a, P: Pattern> MatchIndices<'a, P> {
    #[inline]
    fn match_back(&mut self) -> Option<(usize, &'a str)> {
        match self.window.next_match_back(&mut self.pat, self.haystack) {
            Some((a, b)) => Some((a, unsafe { self.haystack.get_unchecked(a..b) })),
            None => None,
        }
    }
}

impl<'a, P: super::pattern::DoubleEndedPattern> DoubleEndedIterator for MatchIndices<'a, P> {
    #[inline]
    fn next_back(&mut self) -> Option<(usize, &'a str)> {
        self.match_back()
    }
}

impl<'a, P: Pattern> FusedIterator for MatchIndices<'a, P> {}

/// Created with the method `rmatch_indices`.
#[must_use]
pub struct RMatchIndices<'a, P: Pattern>(MatchIndices<'a, P>);

impl<'a, P: Pattern> RMatchIndices<'a, P> {
    #[inline]
    pub(crate) fn new(inner: MatchIndices<'a, P>) -> RMatchIndices<'a, P> {
        RMatchIndices(inner)
    }
}

impl<'a, P: Pattern> Iterator for RMatchIndices<'a, P> {
    type Item = (usize, &'a str);
    #[inline]
    fn next(&mut self) -> Option<(usize, &'a str)> {
        self.0.match_back()
    }
}

impl<'a, P: super::pattern::DoubleEndedPattern> DoubleEndedIterator for RMatchIndices<'a, P> {
    #[inline]
    fn next_back(&mut self) -> Option<(usize, &'a str)> {
        self.0.next()
    }
}

// ---------------------------------------------------------------------------
// Lines / whitespace splitting

/// An iterator over the lines of a string, as string slices.
#[derive(Clone)]
#[must_use]
pub struct Lines<'a>(SplitInclusive<'a, char>);

impl<'a> Lines<'a> {
    #[inline]
    pub(crate) fn new(s: &'a str) -> Lines<'a> {
        Lines(SplitInclusive::new(s, '\n'))
    }

    /// Returns the remaining lines of the split string.
    #[inline]
    pub fn remainder(&self) -> Option<&'a str> {
        (self.0).0.remainder()
    }
}

#[inline]
fn strip_line_ending(line: &str) -> &str {
    let n = line.len();
    let b = line.as_bytes();
    if n > 0 && b[n - 1] == b'\n' {
        if n > 1 && b[n - 2] == b'\r' {
            unsafe { line.get_unchecked(0..n - 2) }
        } else {
            unsafe { line.get_unchecked(0..n - 1) }
        }
    } else {
        line
    }
}

impl<'a> Iterator for Lines<'a> {
    type Item = &'a str;
    #[inline]
    fn next(&mut self) -> Option<&'a str> {
        match self.0.next() {
            Some(line) => Some(strip_line_ending(line)),
            None => None,
        }
    }
}

impl<'a> DoubleEndedIterator for Lines<'a> {
    #[inline]
    fn next_back(&mut self) -> Option<&'a str> {
        match self.0.next_back() {
            Some(line) => Some(strip_line_ending(line)),
            None => None,
        }
    }
}

impl<'a> FusedIterator for Lines<'a> {}

impl<'a> fmt::Debug for Lines<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Lines")
    }
}

/// A pattern matching Unicode whitespace.
#[derive(Clone, Copy, Debug)]
pub(crate) struct IsWhitespace;

impl super::pattern::CharPredicate for IsWhitespace {
    #[inline]
    fn matches_char(&mut self, c: char) -> bool {
        c.is_whitespace()
    }
}

/// A pattern matching ASCII whitespace.
#[derive(Clone, Copy, Debug)]
pub(crate) struct IsAsciiWhitespace;

impl super::pattern::CharPredicate for IsAsciiWhitespace {
    #[inline]
    fn matches_char(&mut self, c: char) -> bool {
        c.is_ascii_whitespace()
    }
}

macro_rules! __std_str_ws_pattern {
    ($($t:ty)*) => {
        $(
            impl super::pattern::DoubleEndedPattern for $t {}
            impl Pattern for $t {
                #[inline]
                fn find_in(&mut self, haystack: &str, start: usize, end: usize) -> Option<(usize, usize)> {
                    let mut i = start;
                    while i < end {
                        let (c, len) = decode_at(haystack, i);
                        if super::pattern::CharPredicate::matches_char(self, c) {
                            return Some((i, i + len));
                        }
                        i += len;
                    }
                    None
                }
                #[inline]
                fn rfind_in(&mut self, haystack: &str, start: usize, end: usize) -> Option<(usize, usize)> {
                    let mut i = end;
                    while i > start {
                        let (c, len) = decode_before(haystack, i);
                        if super::pattern::CharPredicate::matches_char(self, c) {
                            return Some((i - len, i));
                        }
                        i -= len;
                    }
                    None
                }
                #[inline]
                fn prefix_match(&mut self, haystack: &str, start: usize, end: usize) -> Option<usize> {
                    if start < end {
                        let (c, len) = decode_at(haystack, start);
                        if super::pattern::CharPredicate::matches_char(self, c) {
                            return Some(start + len);
                        }
                    }
                    None
                }
                #[inline]
                fn suffix_match(&mut self, haystack: &str, start: usize, end: usize) -> Option<usize> {
                    if start < end {
                        let (c, len) = decode_before(haystack, end);
                        if super::pattern::CharPredicate::matches_char(self, c) {
                            return Some(end - len);
                        }
                    }
                    None
                }
            }
        )*
    };
}

__std_str_ws_pattern! { IsWhitespace IsAsciiWhitespace }

/// An iterator over the non-whitespace substrings of a string, separated by any amount of whitespace.
#[derive(Clone)]
#[must_use]
pub struct SplitWhitespace<'a> {
    inner: Split<'a, IsWhitespace>,
}

impl<'a> SplitWhitespace<'a> {
    #[inline]
    pub(crate) fn new(s: &'a str) -> SplitWhitespace<'a> {
        SplitWhitespace { inner: Split::new(s, IsWhitespace, true) }
    }

    /// Returns remainder of the split string.
    #[inline]
    pub fn remainder(&self) -> Option<&'a str> {
        match self.inner.remainder() {
            Some(r) => {
                let t = r.trim_start();
                if t.is_empty() {
                    None
                } else {
                    Some(t)
                }
            }
            None => None,
        }
    }
}

impl<'a> Iterator for SplitWhitespace<'a> {
    type Item = &'a str;
    #[inline]
    fn next(&mut self) -> Option<&'a str> {
        while let Some(s) = self.inner.next_fwd() {
            if !s.is_empty() {
                return Some(s);
            }
        }
        None
    }
    #[inline]
    fn last(self) -> Option<&'a str> {
        let mut s = self;
        s.next_back()
    }
}

impl<'a> DoubleEndedIterator for SplitWhitespace<'a> {
    #[inline]
    fn next_back(&mut self) -> Option<&'a str> {
        while let Some(s) = self.inner.next_bwd() {
            if !s.is_empty() {
                return Some(s);
            }
        }
        None
    }
}

impl<'a> FusedIterator for SplitWhitespace<'a> {}

impl<'a> fmt::Debug for SplitWhitespace<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SplitWhitespace")
    }
}

/// An iterator over the non-ASCII-whitespace substrings of a string, separated by any amount of ASCII whitespace.
#[derive(Clone)]
#[must_use]
pub struct SplitAsciiWhitespace<'a> {
    inner: Split<'a, IsAsciiWhitespace>,
}

impl<'a> SplitAsciiWhitespace<'a> {
    #[inline]
    pub(crate) fn new(s: &'a str) -> SplitAsciiWhitespace<'a> {
        SplitAsciiWhitespace { inner: Split::new(s, IsAsciiWhitespace, true) }
    }
}

impl<'a> Iterator for SplitAsciiWhitespace<'a> {
    type Item = &'a str;
    #[inline]
    fn next(&mut self) -> Option<&'a str> {
        while let Some(s) = self.inner.next_fwd() {
            if !s.is_empty() {
                return Some(s);
            }
        }
        None
    }
}

impl<'a> DoubleEndedIterator for SplitAsciiWhitespace<'a> {
    #[inline]
    fn next_back(&mut self) -> Option<&'a str> {
        while let Some(s) = self.inner.next_bwd() {
            if !s.is_empty() {
                return Some(s);
            }
        }
        None
    }
}

impl<'a> FusedIterator for SplitAsciiWhitespace<'a> {}

// ---------------------------------------------------------------------------
// EncodeUtf16

/// An iterator of `u16` over the string encoded as UTF-16.
#[derive(Clone)]
#[must_use]
pub struct EncodeUtf16<'a> {
    chars: Chars<'a>,
    extra: u16,
}

impl<'a> EncodeUtf16<'a> {
    #[inline]
    pub(crate) fn new(s: &'a str) -> EncodeUtf16<'a> {
        EncodeUtf16 { chars: s.chars(), extra: 0 }
    }
}

impl<'a> Iterator for EncodeUtf16<'a> {
    type Item = u16;
    #[inline]
    fn next(&mut self) -> Option<u16> {
        if self.extra != 0 {
            let tmp = self.extra;
            self.extra = 0;
            return Some(tmp);
        }
        match self.chars.next() {
            Some(ch) => {
                let mut code = ch as u32;
                if code < 0x10000 {
                    Some(code as u16)
                } else {
                    code -= 0x10000;
                    self.extra = 0xDC00 | ((code as u16) & 0x3FF);
                    Some(0xD800 | ((code >> 10) as u16))
                }
            }
            None => None,
        }
    }
}

impl<'a> FusedIterator for EncodeUtf16<'a> {}

// ---------------------------------------------------------------------------
// EscapeDebug / EscapeDefault / EscapeUnicode
//
// The escaped text is produced eagerly; the iterator walks over its chars and
// `Display` writes the remainder.

macro_rules! str_escape_iter {
    ($name:ident) => {
        /// The iterator returned by the corresponding `str::escape_*` method.
        #[derive(Clone)]
        pub struct $name<'a> {
            s: crate::string::String,
            pos: usize,
            _m: crate::marker::PhantomData<&'a str>,
        }

        impl<'a> $name<'a> {
            pub(crate) fn new(s: crate::string::String) -> $name<'a> {
                $name { s, pos: 0, _m: crate::marker::PhantomData }
            }
        }

        impl<'a> Iterator for $name<'a> {
            type Item = char;
            #[inline]
            fn next(&mut self) -> Option<char> {
                if self.pos < self.s.len() {
                    let (c, len) = decode_at(self.s.as_str(), self.pos);
                    self.pos += len;
                    Some(c)
                } else {
                    None
                }
            }
            // size_hint: the default (0, None), like std's flattening iterators.
        }

        impl<'a> FusedIterator for $name<'a> {}

        impl<'a> fmt::Display for $name<'a> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(unsafe { self.s.as_str().get_unchecked(self.pos..self.s.len()) })
            }
        }

        impl<'a> fmt::Debug for $name<'a> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_struct(stringify!($name)).finish_non_exhaustive()
            }
        }
    };
}

str_escape_iter!(EscapeDebug);
str_escape_iter!(EscapeDefault);
str_escape_iter!(EscapeUnicode);
