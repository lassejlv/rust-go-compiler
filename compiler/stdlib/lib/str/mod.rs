//! String slices (`str`).

pub mod pattern;
mod iter;
mod traits;
mod validation;

pub use self::iter::{
    Bytes, CharIndices, Chars, EncodeUtf16, Lines, MatchIndices, Matches, RMatchIndices, RMatches, RSplit, RSplitN,
    RSplitTerminator, Split, SplitAsciiWhitespace, SplitInclusive, SplitN, SplitTerminator, SplitWhitespace,
};
pub use self::traits::{FromStr, ParseBoolError};
pub use self::validation::{from_utf8, from_utf8_mut, from_utf8_unchecked, from_utf8_unchecked_mut, Utf8Error};
pub use crate::slice::SliceIndex;

use self::pattern::{Pattern, SearchWindow};
use crate::intrinsics;

// ---------------------------------------------------------------------------
// UTF-8 helpers (the input is always valid UTF-8)

/// Decodes the char starting at byte `i` of `s`: (char, encoded length).
#[inline]
pub(crate) fn decode_at(s: &str, i: usize) -> (char, usize) {
    let b = s.as_bytes();
    let x = b[i] as u32;
    if x < 0x80 {
        return (unsafe { crate::char::from_u32_unchecked(x) }, 1);
    }
    if x < 0xE0 {
        let c = ((x & 0x1F) << 6) | (b[i + 1] as u32 & 0x3F);
        return (unsafe { crate::char::from_u32_unchecked(c) }, 2);
    }
    if x < 0xF0 {
        let c = ((x & 0x0F) << 12) | ((b[i + 1] as u32 & 0x3F) << 6) | (b[i + 2] as u32 & 0x3F);
        return (unsafe { crate::char::from_u32_unchecked(c) }, 3);
    }
    let c = ((x & 0x07) << 18) | ((b[i + 1] as u32 & 0x3F) << 12) | ((b[i + 2] as u32 & 0x3F) << 6) | (b[i + 3] as u32 & 0x3F);
    (unsafe { crate::char::from_u32_unchecked(c) }, 4)
}

/// Decodes the char ending just before byte `i` of `s`: (char, encoded length).
#[inline]
pub(crate) fn decode_before(s: &str, i: usize) -> (char, usize) {
    let b = s.as_bytes();
    let mut start = i - 1;
    while (b[start] & 0xC0) == 0x80 {
        start -= 1;
    }
    let (c, len) = decode_at(s, start);
    (c, len)
}

#[inline]
pub(crate) unsafe fn from_raw_parts<'a>(p: *const u8, len: usize) -> &'a str {
    &*intrinsics::str_from_raw_parts(p, len)
}

#[inline]
pub(crate) unsafe fn from_raw_parts_mut<'a>(p: *mut u8, len: usize) -> &'a mut str {
    &mut *(intrinsics::str_from_raw_parts(p as *const u8, len) as *mut str)
}

#[inline(never)]
#[track_caller]
pub(crate) fn slice_error_fail(s: &str, begin: usize, end: usize) -> ! {
    let len = s.len();
    // 1. begin is OOB.
    if begin > len {
        crate::rt::panic_fmt(
            format_args!("start byte index {begin} is out of bounds for string of length {len}"),
            file!(),
            line!(),
            column!(),
        );
    }
    // 2. end is OOB.
    if end > len {
        crate::rt::panic_fmt(
            format_args!("end byte index {end} is out of bounds for string of length {len}"),
            file!(),
            line!(),
            column!(),
        );
    }
    // 3. range is backwards.
    if begin > end {
        crate::rt::panic_fmt(format_args!("byte range starts at {begin} but ends at {end}"), file!(), line!(), column!());
    }
    // 4. begin is inside a character.
    if !s.is_char_boundary(begin) {
        let floor = s.floor_char_boundary(begin);
        let (ch, ch_len) = decode_at(s, floor);
        let range = floor..floor + ch_len;
        crate::rt::panic_fmt(
            format_args!("start byte index {begin} is not a char boundary; it is inside {ch:?} (bytes {range:?} of string)"),
            file!(),
            line!(),
            column!(),
        );
    }
    // 5. end is inside a character.
    if !s.is_char_boundary(end) {
        let floor = s.floor_char_boundary(end);
        let (ch, ch_len) = decode_at(s, floor);
        let range = floor..floor + ch_len;
        crate::rt::panic_fmt(
            format_args!("end byte index {end} is not a char boundary; it is inside {ch:?} (bytes {range:?} of string)"),
            file!(),
            line!(),
            column!(),
        );
    }
    // 6. end is OOB and the range is inclusive (end == len).
    crate::rt::panic_fmt(
        format_args!("end byte index {end} is out of bounds for string of length {len}"),
        file!(),
        line!(),
        column!(),
    )
}

// Unicode helpers for case conversion.
fn is_case_ignorable(c: char) -> bool {
    match c {
        '\'' | '.' | ':' | '^' | '`' | '\u{B7}' | '\u{2019}' => true,
        _ => crate::char::is_grapheme_extended(c),
    }
}

fn is_cased(c: char) -> bool {
    c.is_lowercase() || c.is_uppercase()
}

impl str {
    /// Returns the length of `self` in bytes.
    #[inline]
    pub const fn len(&self) -> usize {
        unsafe { intrinsics::str_len(self as *const str) }
    }

    /// Returns `true` if `self` has a length of zero bytes.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Checks that `index`-th byte is the first byte in a UTF-8 code point sequence or the end of the string.
    #[inline]
    pub fn is_char_boundary(&self, index: usize) -> bool {
        if index == 0 {
            return true;
        }
        let len = self.len();
        if index >= len {
            index == len
        } else {
            (self.as_bytes()[index] as i8) >= -0x40
        }
    }

    /// Finds the closest `x` not exceeding `index` where `is_char_boundary(x)` is `true`.
    #[inline]
    pub fn floor_char_boundary(&self, index: usize) -> usize {
        if index >= self.len() {
            self.len()
        } else {
            let mut i = index;
            while !self.is_char_boundary(i) {
                i -= 1;
            }
            i
        }
    }

    /// Finds the closest `x` not below `index` where `is_char_boundary(x)` is `true`.
    #[inline]
    pub fn ceil_char_boundary(&self, index: usize) -> usize {
        if index > self.len() {
            self.len()
        } else {
            let mut i = index;
            while !self.is_char_boundary(i) {
                i += 1;
            }
            i
        }
    }

    /// Converts a string slice to a byte slice.
    #[inline]
    pub const fn as_bytes(&self) -> &[u8] {
        unsafe {
            let p = self as *const str;
            &*intrinsics::slice_from_raw_parts(intrinsics::str_ptr(p), intrinsics::str_len(p))
        }
    }

    /// Converts a mutable string slice to a mutable byte slice.
    #[inline]
    pub unsafe fn as_bytes_mut(&mut self) -> &mut [u8] {
        let p = self as *mut str as *const str;
        &mut *intrinsics::slice_from_raw_parts_mut(intrinsics::str_ptr(p) as *mut u8, intrinsics::str_len(p))
    }

    /// Converts a string slice to a raw pointer.
    #[inline]
    pub const fn as_ptr(&self) -> *const u8 {
        unsafe { intrinsics::str_ptr(self as *const str) }
    }

    /// Converts a mutable string slice to a raw pointer.
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        unsafe { intrinsics::str_ptr(self as *mut str as *const str) as *mut u8 }
    }

    /// Returns a subslice of `str`.
    #[inline]
    pub fn get<I: SliceIndex<str>>(&self, i: I) -> Option<&I::Output> {
        i.get(self)
    }

    /// Returns a mutable subslice of `str`.
    #[inline]
    pub fn get_mut<I: SliceIndex<str>>(&mut self, i: I) -> Option<&mut I::Output> {
        i.get_mut(self)
    }

    /// Returns an unchecked subslice of `str`.
    #[inline]
    pub unsafe fn get_unchecked<I: SliceIndex<str>>(&self, i: I) -> &I::Output {
        &*i.get_unchecked(self as *const str)
    }

    /// Returns a mutable, unchecked subslice of `str`.
    #[inline]
    pub unsafe fn get_unchecked_mut<I: SliceIndex<str>>(&mut self, i: I) -> &mut I::Output {
        &mut *i.get_unchecked_mut(self as *mut str)
    }

    /// Creates a string slice from another string slice, bypassing safety checks.
    #[inline]
    pub unsafe fn slice_unchecked(&self, begin: usize, end: usize) -> &str {
        from_raw_parts(self.as_ptr().add(begin), end - begin)
    }

    /// Divides one string slice into two at an index.
    #[inline]
    #[track_caller]
    pub fn split_at(&self, mid: usize) -> (&str, &str) {
        match self.split_at_checked(mid) {
            Some(pair) => pair,
            None => slice_error_fail(self, 0, mid),
        }
    }

    /// Divides one mutable string slice into two at an index.
    #[inline]
    #[track_caller]
    pub fn split_at_mut(&mut self, mid: usize) -> (&mut str, &mut str) {
        if self.is_char_boundary(mid) {
            let len = self.len();
            let p = self.as_mut_ptr();
            unsafe { (from_raw_parts_mut(p, mid), from_raw_parts_mut(p.add(mid), len - mid)) }
        } else {
            slice_error_fail(self, 0, mid)
        }
    }

    /// Divides one mutable string slice into two at an index, returning `None` if the index is not on a char boundary.
    #[inline]
    pub fn split_at_mut_checked(&mut self, mid: usize) -> Option<(&mut str, &mut str)> {
        if self.is_char_boundary(mid) {
            let len = self.len();
            let p = self.as_mut_ptr();
            unsafe { Some((from_raw_parts_mut(p, mid), from_raw_parts_mut(p.add(mid), len - mid))) }
        } else {
            None
        }
    }

    /// Divides one string slice into two at an index, returning `None` if the index is not on a char boundary.
    #[inline]
    pub fn split_at_checked(&self, mid: usize) -> Option<(&str, &str)> {
        if self.is_char_boundary(mid) {
            let len = self.len();
            let p = self.as_ptr();
            unsafe { Some((from_raw_parts(p, mid), from_raw_parts(p.add(mid), len - mid))) }
        } else {
            None
        }
    }

    /// Returns an iterator over the `char`s of a string slice.
    #[inline]
    pub fn chars(&self) -> Chars<'_> {
        Chars::new(self)
    }

    /// Returns an iterator over the `char`s of a string slice, and their positions.
    #[inline]
    pub fn char_indices(&self) -> CharIndices<'_> {
        CharIndices::new(self)
    }

    /// Returns an iterator over the bytes of a string slice.
    #[inline]
    pub fn bytes(&self) -> Bytes<'_> {
        Bytes::new(self.as_bytes())
    }

    /// Splits a string slice by whitespace.
    #[inline]
    pub fn split_whitespace(&self) -> SplitWhitespace<'_> {
        SplitWhitespace::new(self)
    }

    /// Splits a string slice by ASCII whitespace.
    #[inline]
    pub fn split_ascii_whitespace(&self) -> SplitAsciiWhitespace<'_> {
        SplitAsciiWhitespace::new(self)
    }

    /// Returns an iterator over the lines of a string, as string slices.
    #[inline]
    pub fn lines(&self) -> Lines<'_> {
        Lines::new(self)
    }

    /// Returns an iterator of `u16` over the string encoded as UTF-16.
    #[inline]
    pub fn encode_utf16(&self) -> EncodeUtf16<'_> {
        EncodeUtf16::new(self)
    }

    /// Returns `true` if the given pattern matches a sub-slice of this string slice.
    #[inline]
    pub fn contains<P: Pattern>(&self, pat: P) -> bool {
        pat.is_contained_in(self)
    }

    /// Returns `true` if the given pattern matches a prefix of this string slice.
    #[inline]
    pub fn starts_with<P: Pattern>(&self, pat: P) -> bool {
        pat.is_prefix_of(self)
    }

    /// Returns `true` if the given pattern matches a suffix of this string slice.
    #[inline]
    pub fn ends_with<P: Pattern>(&self, pat: P) -> bool {
        pat.is_suffix_of(self)
    }

    /// Returns the byte index of the first character of this string slice that matches the pattern.
    #[inline]
    pub fn find<P: Pattern>(&self, pat: P) -> Option<usize> {
        let mut pat = pat;
        match pat.find_in(self, 0, self.len()) {
            Some((a, _)) => Some(a),
            None => None,
        }
    }

    /// Returns the byte index for the first character of the last match of the pattern in this string slice.
    #[inline]
    pub fn rfind<P: Pattern>(&self, pat: P) -> Option<usize> {
        let mut pat = pat;
        match pat.rfind_in(self, 0, self.len()) {
            Some((a, _)) => Some(a),
            None => None,
        }
    }

    /// Returns an iterator over substrings of this string slice, separated by characters matched by a pattern.
    #[inline]
    pub fn split<P: Pattern>(&self, pat: P) -> Split<'_, P> {
        Split::new(self, pat, true)
    }

    /// Returns an iterator over substrings of this string slice, separated by characters matched by a pattern (the matched part is included at the end of each substring).
    #[inline]
    pub fn split_inclusive<P: Pattern>(&self, pat: P) -> SplitInclusive<'_, P> {
        SplitInclusive::new(self, pat)
    }

    /// Returns an iterator over substrings of the given string slice, separated by characters matched by a pattern and yielded in reverse order.
    #[inline]
    pub fn rsplit<P: Pattern>(&self, pat: P) -> RSplit<'_, P> {
        RSplit::new(Split::new(self, pat, true))
    }

    /// Like `split`, but a trailing empty substring is skipped.
    #[inline]
    pub fn split_terminator<P: Pattern>(&self, pat: P) -> SplitTerminator<'_, P> {
        SplitTerminator::new(Split::new(self, pat, false))
    }

    /// Like `rsplit`, but a trailing empty substring is skipped.
    #[inline]
    pub fn rsplit_terminator<P: Pattern>(&self, pat: P) -> RSplitTerminator<'_, P> {
        RSplitTerminator::new(Split::new(self, pat, false))
    }

    /// Returns an iterator over substrings of the given string slice, separated by a pattern, restricted to returning at most `n` items.
    #[inline]
    pub fn splitn<P: Pattern>(&self, n: usize, pat: P) -> SplitN<'_, P> {
        SplitN::new(Split::new(self, pat, true), n)
    }

    /// Like `splitn`, starting from the end of the string.
    #[inline]
    pub fn rsplitn<P: Pattern>(&self, n: usize, pat: P) -> RSplitN<'_, P> {
        RSplitN::new(Split::new(self, pat, true), n)
    }

    /// Splits the string on the first occurrence of the specified delimiter.
    #[inline]
    pub fn split_once<P: Pattern>(&self, delimiter: P) -> Option<(&str, &str)> {
        let mut delimiter = delimiter;
        match delimiter.find_in(self, 0, self.len()) {
            Some((start, end)) => unsafe { Some((self.get_unchecked(0..start), self.get_unchecked(end..self.len()))) },
            None => None,
        }
    }

    /// Splits the string on the last occurrence of the specified delimiter.
    #[inline]
    pub fn rsplit_once<P: Pattern>(&self, delimiter: P) -> Option<(&str, &str)> {
        let mut delimiter = delimiter;
        match delimiter.rfind_in(self, 0, self.len()) {
            Some((start, end)) => unsafe { Some((self.get_unchecked(0..start), self.get_unchecked(end..self.len()))) },
            None => None,
        }
    }

    /// Returns an iterator over the disjoint matches of a pattern within the given string slice.
    #[inline]
    pub fn matches<P: Pattern>(&self, pat: P) -> Matches<'_, P> {
        Matches::new(self, pat)
    }

    /// Returns an iterator over the disjoint matches of a pattern within this string slice, yielded in reverse order.
    #[inline]
    pub fn rmatches<P: Pattern>(&self, pat: P) -> RMatches<'_, P> {
        RMatches::new(Matches::new(self, pat))
    }

    /// Returns an iterator over the disjoint matches of a pattern within this string slice as well as the index that the match starts at.
    #[inline]
    pub fn match_indices<P: Pattern>(&self, pat: P) -> MatchIndices<'_, P> {
        MatchIndices::new(self, pat)
    }

    /// Like `match_indices`, yielded in reverse order.
    #[inline]
    pub fn rmatch_indices<P: Pattern>(&self, pat: P) -> RMatchIndices<'_, P> {
        RMatchIndices::new(MatchIndices::new(self, pat))
    }

    /// Returns a string slice with leading and trailing whitespace removed.
    #[inline]
    pub fn trim(&self) -> &str {
        self.trim_matches(|c: char| c.is_whitespace())
    }

    /// Returns a string slice with leading whitespace removed.
    #[inline]
    pub fn trim_start(&self) -> &str {
        self.trim_start_matches(|c: char| c.is_whitespace())
    }

    /// Returns a string slice with trailing whitespace removed.
    #[inline]
    pub fn trim_end(&self) -> &str {
        self.trim_end_matches(|c: char| c.is_whitespace())
    }

    /// Returns a string slice with leading whitespace removed (deprecated alias).
    #[inline]
    pub fn trim_left(&self) -> &str {
        self.trim_start()
    }

    /// Returns a string slice with trailing whitespace removed (deprecated alias).
    #[inline]
    pub fn trim_right(&self) -> &str {
        self.trim_end()
    }

    /// Returns a string slice with leading and trailing ASCII whitespace removed.
    #[inline]
    pub fn trim_ascii(&self) -> &str {
        self.trim_matches(|c: char| c.is_ascii_whitespace())
    }

    /// Returns a string slice with leading ASCII whitespace removed.
    #[inline]
    pub fn trim_ascii_start(&self) -> &str {
        self.trim_start_matches(|c: char| c.is_ascii_whitespace())
    }

    /// Returns a string slice with trailing ASCII whitespace removed.
    #[inline]
    pub fn trim_ascii_end(&self) -> &str {
        self.trim_end_matches(|c: char| c.is_ascii_whitespace())
    }

    /// Returns a string slice with all prefixes and suffixes that match a pattern repeatedly removed.
    #[inline]
    pub fn trim_matches<P: Pattern>(&self, pat: P) -> &str {
        let mut pat = pat;
        let mut i = 0;
        let mut j = self.len();
        loop {
            match pat.prefix_match(self, i, j) {
                Some(e) if e > i => i = e,
                _ => break,
            }
        }
        loop {
            match pat.suffix_match(self, i, j) {
                Some(s) if s < j => j = s,
                _ => break,
            }
        }
        unsafe { self.get_unchecked(i..j) }
    }

    /// Returns a string slice with all prefixes that match a pattern repeatedly removed.
    #[inline]
    pub fn trim_start_matches<P: Pattern>(&self, pat: P) -> &str {
        let mut pat = pat;
        let mut i = 0;
        let j = self.len();
        loop {
            match pat.prefix_match(self, i, j) {
                Some(e) if e > i => i = e,
                _ => break,
            }
        }
        unsafe { self.get_unchecked(i..j) }
    }

    /// Returns a string slice with all suffixes that match a pattern repeatedly removed.
    #[inline]
    pub fn trim_end_matches<P: Pattern>(&self, pat: P) -> &str {
        let mut pat = pat;
        let i = 0;
        let mut j = self.len();
        loop {
            match pat.suffix_match(self, i, j) {
                Some(s) if s < j => j = s,
                _ => break,
            }
        }
        unsafe { self.get_unchecked(i..j) }
    }

    /// Deprecated alias of `trim_start_matches`.
    #[inline]
    pub fn trim_left_matches<P: Pattern>(&self, pat: P) -> &str {
        self.trim_start_matches(pat)
    }

    /// Deprecated alias of `trim_end_matches`.
    #[inline]
    pub fn trim_right_matches<P: Pattern>(&self, pat: P) -> &str {
        self.trim_end_matches(pat)
    }

    /// Returns a string slice with the prefix removed.
    #[inline]
    pub fn strip_prefix<P: Pattern>(&self, prefix: P) -> Option<&str> {
        prefix.strip_prefix_of(self)
    }

    /// Returns a string slice with the suffix removed.
    #[inline]
    pub fn strip_suffix<P: Pattern>(&self, suffix: P) -> Option<&str> {
        suffix.strip_suffix_of(self)
    }

    /// Parses this string slice into another type.
    #[inline]
    pub fn parse<F: FromStr>(&self) -> Result<F, F::Err> {
        FromStr::from_str(self)
    }

    /// Checks if all characters in this string are within the ASCII range.
    #[inline]
    pub fn is_ascii(&self) -> bool {
        let b = self.as_bytes();
        let n = b.len();
        let mut i = 0;
        while i < n {
            if b[i] >= 0x80 {
                return false;
            }
            i += 1;
        }
        true
    }

    /// Checks that two strings are an ASCII case-insensitive match.
    #[inline]
    pub fn eq_ignore_ascii_case(&self, other: &str) -> bool {
        let a = self.as_bytes();
        let b = other.as_bytes();
        if a.len() != b.len() {
            return false;
        }
        let n = a.len();
        let mut i = 0;
        while i < n {
            if a[i].to_ascii_lowercase() != b[i].to_ascii_lowercase() {
                return false;
            }
            i += 1;
        }
        true
    }

    /// Converts this string to its ASCII upper case equivalent in-place.
    #[inline]
    pub fn make_ascii_uppercase(&mut self) {
        let b = unsafe { self.as_bytes_mut() };
        let n = b.len();
        let mut i = 0;
        while i < n {
            b[i] = b[i].to_ascii_uppercase();
            i += 1;
        }
    }

    /// Converts this string to its ASCII lower case equivalent in-place.
    #[inline]
    pub fn make_ascii_lowercase(&mut self) {
        let b = unsafe { self.as_bytes_mut() };
        let n = b.len();
        let mut i = 0;
        while i < n {
            b[i] = b[i].to_ascii_lowercase();
            i += 1;
        }
    }

    /// Returns a copy of this string where each character is mapped to its ASCII upper case equivalent.
    #[inline]
    pub fn to_ascii_uppercase(&self) -> String {
        let mut s = String::from(self);
        s.make_ascii_uppercase();
        s
    }

    /// Returns a copy of this string where each character is mapped to its ASCII lower case equivalent.
    #[inline]
    pub fn to_ascii_lowercase(&self) -> String {
        let mut s = String::from(self);
        s.make_ascii_lowercase();
        s
    }

    /// Returns the lowercase equivalent of this string slice, as a new `String`.
    pub fn to_lowercase(&self) -> String {
        let mut s = String::with_capacity(self.len());
        let len = self.len();
        let mut i = 0;
        while i < len {
            let (c, clen) = decode_at(self, i);
            if c == 'Σ' {
                // Final sigma rule.
                let before = unsafe { self.get_unchecked(0..i) };
                let after = unsafe { self.get_unchecked(i + clen..len) };
                let mut is_word_final = false;
                let mut it = before.chars().rev();
                while let Some(p) = it.next() {
                    if is_case_ignorable(p) {
                        continue;
                    }
                    is_word_final = is_cased(p);
                    break;
                }
                if is_word_final {
                    let mut it2 = after.chars();
                    while let Some(n) = it2.next() {
                        if is_case_ignorable(n) {
                            continue;
                        }
                        if is_cased(n) {
                            is_word_final = false;
                        }
                        break;
                    }
                }
                s.push(if is_word_final { 'ς' } else { 'σ' });
            } else {
                let mut m = c.to_lowercase();
                while let Some(x) = m.next() {
                    s.push(x);
                }
            }
            i += clen;
        }
        s
    }

    /// Returns the uppercase equivalent of this string slice, as a new `String`.
    pub fn to_uppercase(&self) -> String {
        let mut s = String::with_capacity(self.len());
        let len = self.len();
        let mut i = 0;
        while i < len {
            let (c, clen) = decode_at(self, i);
            let mut m = c.to_uppercase();
            while let Some(x) = m.next() {
                s.push(x);
            }
            i += clen;
        }
        s
    }

    /// Creates a new `String` by repeating a string `n` times.
    pub fn repeat(&self, n: usize) -> String {
        let total = match self.len().checked_mul(n) {
            Some(t) => t,
            None => crate::rt::panic_str("capacity overflow", file!(), line!(), column!()),
        };
        let mut s = String::with_capacity(total);
        let mut i = 0;
        while i < n {
            s.push_str(self);
            i += 1;
        }
        s
    }

    /// Replaces all matches of a pattern with another string.
    pub fn replace<P: Pattern>(&self, from: P, to: &str) -> String {
        let mut result = String::with_capacity(self.len());
        let mut last_end = 0;
        let mut pat = from;
        let mut w = SearchWindow::new(self.len());
        while let Some((start, end)) = w.next_match(&mut pat, self) {
            result.push_str(unsafe { self.get_unchecked(last_end..start) });
            result.push_str(to);
            last_end = end;
        }
        result.push_str(unsafe { self.get_unchecked(last_end..self.len()) });
        result
    }

    /// Replaces first N matches of a pattern with another string.
    pub fn replacen<P: Pattern>(&self, pat: P, to: &str, count: usize) -> String {
        let mut result = String::with_capacity(32);
        let mut last_end = 0;
        let mut pat = pat;
        let mut w = SearchWindow::new(self.len());
        let mut n = 0;
        while n < count {
            match w.next_match(&mut pat, self) {
                Some((start, end)) => {
                    result.push_str(unsafe { self.get_unchecked(last_end..start) });
                    result.push_str(to);
                    last_end = end;
                }
                None => break,
            }
            n += 1;
        }
        result.push_str(unsafe { self.get_unchecked(last_end..self.len()) });
        result
    }

    /// Converts a `Box<str>` into a `String` without copying or allocating.
    pub fn into_string(self: Box<str>) -> String {
        String::from(self)
    }

    /// Converts a `Box<str>` into a `Box<[u8]>` without copying or allocating.
    pub fn into_boxed_bytes(self: Box<str>) -> Box<[u8]> {
        String::from(self).into_bytes().into_boxed_slice()
    }

    /// Returns an iterator that escapes each char in `self` with `char::escape_debug`.
    pub fn escape_debug(&self) -> String {
        let mut s = String::new();
        let mut first = true;
        let mut it = self.chars();
        while let Some(c) = it.next() {
            if first {
                // Only the first char is escaped if it is a grapheme extender.
                let mut e = c.escape_debug();
                while let Some(x) = e.next() {
                    s.push(x);
                }
                first = false;
            } else if c == '\'' || c == '"' || c == '\\' || c == '\n' || c == '\r' || c == '\t' || c == '\0' || !crate::char::is_printable(c) {
                let mut e = c.escape_debug();
                while let Some(x) = e.next() {
                    s.push(x);
                }
            } else {
                s.push(c);
            }
        }
        s
    }

    /// Returns an iterator that escapes each char in `self` with `char::escape_default`.
    pub fn escape_default(&self) -> String {
        let mut s = String::new();
        let mut it = self.chars();
        while let Some(c) = it.next() {
            let mut e = c.escape_default();
            while let Some(x) = e.next() {
                s.push(x);
            }
        }
        s
    }

    /// Returns an iterator that escapes each char in `self` with `char::escape_unicode`.
    pub fn escape_unicode(&self) -> String {
        let mut s = String::new();
        let mut it = self.chars();
        while let Some(c) = it.next() {
            let mut e = c.escape_unicode();
            while let Some(x) = e.next() {
                s.push(x);
            }
        }
        s
    }
}
