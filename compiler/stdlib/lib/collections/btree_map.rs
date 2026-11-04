//! An ordered map, implemented as a sorted sequence of bounded sorted blocks
//! (a two-level "B-tree" of height 2): lookups binary-search the blocks and
//! then the block, inserts and removals move at most one block's elements
//! (and occasionally the block list), so all operations stay fast for large
//! maps (a plain sorted vector made random inserts and `pop_first` O(n)).

use crate::borrow::Borrow;
use crate::cmp::Ordering;
use crate::fmt;
use crate::hash::{Hash, Hasher};
use crate::iter::{DoubleEndedIterator, ExactSizeIterator, Extend, FromIterator, FusedIterator, IntoIterator, Iterator};
use crate::mem;
use crate::ops::{Bound, Index, RangeBounds};
use crate::slice;
use crate::vec;

/// Blocks are split when they grow beyond this many entries.
const BLOCK_MAX: usize = 128;

/// An ordered map based on a B-Tree.
pub struct BTreeMap<K, V> {
    // non-empty sorted blocks; their concatenation is sorted by key
    blocks: Vec<Vec<(K, V)>>,
    len: usize,
}

impl<K, V> BTreeMap<K, V> {
    /// Makes a new, empty `BTreeMap`.
    #[inline]
    pub const fn new() -> BTreeMap<K, V> {
        BTreeMap { blocks: Vec::new(), len: 0 }
    }

    /// Clears the map, removing all elements.
    #[inline]
    pub fn clear(&mut self) {
        self.blocks.clear();
        self.len = 0;
    }

    /// Gets an iterator over the entries of the map, sorted by key.
    #[inline]
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter { inner: Flat::new(&self.blocks, self.len) }
    }

    /// Gets a mutable iterator over the entries of the map, sorted by key.
    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        let n = self.len;
        IterMut { inner: FlatMut::new(&mut self.blocks, n) }
    }

    /// Gets an iterator over the keys of the map, in sorted order.
    #[inline]
    pub fn keys(&self) -> Keys<'_, K, V> {
        Keys { inner: self.iter() }
    }

    /// Gets an iterator over the values of the map, in order by key.
    #[inline]
    pub fn values(&self) -> Values<'_, K, V> {
        Values { inner: self.iter() }
    }

    /// Gets a mutable iterator over the values of the map, in order by key.
    #[inline]
    pub fn values_mut(&mut self) -> ValuesMut<'_, K, V> {
        ValuesMut { inner: self.iter_mut() }
    }

    /// Creates a consuming iterator visiting all the keys, in sorted order.
    #[inline]
    pub fn into_keys(self) -> IntoKeys<K, V> {
        IntoKeys { inner: self.into_iter() }
    }

    /// Creates a consuming iterator visiting all the values, in order by key.
    #[inline]
    pub fn into_values(self) -> IntoValues<K, V> {
        IntoValues { inner: self.into_iter() }
    }

    /// Returns the number of elements in the map.
    #[inline]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the map contains no elements.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the first key-value pair in the map (the minimum key).
    #[inline]
    pub fn first_key_value(&self) -> Option<(&K, &V)> {
        match self.blocks.first() {
            Some(b) => {
                let e = &b[0];
                Some((&e.0, &e.1))
            }
            None => None,
        }
    }

    /// Returns the last key-value pair in the map (the maximum key).
    #[inline]
    pub fn last_key_value(&self) -> Option<(&K, &V)> {
        match self.blocks.last() {
            Some(b) => {
                let e = &b[b.len() - 1];
                Some((&e.0, &e.1))
            }
            None => None,
        }
    }

    /// Removes and returns the first element in the map.
    #[inline]
    pub fn pop_first(&mut self) -> Option<(K, V)> {
        if self.len == 0 {
            None
        } else {
            Some(self.remove_at(0, 0))
        }
    }

    /// Removes and returns the last element in the map.
    #[inline]
    pub fn pop_last(&mut self) -> Option<(K, V)> {
        if self.len == 0 {
            return None;
        }
        let b = self.blocks.len() - 1;
        let i = self.blocks[b].len() - 1;
        Some(self.remove_at(b, i))
    }

    /// Returns the first entry in the map for in-place manipulation.
    pub fn first_entry(&mut self) -> Option<OccupiedEntry<'_, K, V>> {
        if self.len == 0 {
            None
        } else {
            Some(OccupiedEntry { map: self, b: 0, i: 0 })
        }
    }

    /// Returns the last entry in the map for in-place manipulation.
    pub fn last_entry(&mut self) -> Option<OccupiedEntry<'_, K, V>> {
        if self.len == 0 {
            return None;
        }
        let b = self.blocks.len() - 1;
        let i = self.blocks[b].len() - 1;
        Some(OccupiedEntry { map: self, b, i })
    }

    /// Retains only the elements specified by the predicate.
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&K, &mut V) -> bool,
    {
        let mut f = f;
        let mut n = 0;
        let mut bi = 0;
        while bi < self.blocks.len() {
            self.blocks[bi].retain_mut(|e| f(&e.0, &mut e.1));
            n += self.blocks[bi].len();
            bi += 1;
        }
        self.blocks.retain(|b| !b.is_empty());
        self.len = n;
    }

    /// Removes the entry at block `b`, index `i`.
    fn remove_at(&mut self, b: usize, i: usize) -> (K, V) {
        let e = self.blocks[b].remove(i);
        self.len -= 1;
        let n = self.blocks[b].len();
        if n == 0 {
            self.blocks.remove(b);
        } else if n < BLOCK_MAX / 4 && b + 1 < self.blocks.len() && n + self.blocks[b + 1].len() <= BLOCK_MAX {
            // merge a small block with its successor
            let next = self.blocks.remove(b + 1);
            self.blocks[b].extend(next);
        }
        e
    }

    /// Inserts at block `b`, index `i` (a valid insertion point), splitting
    /// a full block; returns the entry's final position.
    fn insert_at(&mut self, b: usize, i: usize, key: K, value: V) -> (usize, usize) {
        self.len += 1;
        if self.blocks.is_empty() {
            let mut blk = Vec::with_capacity(8);
            blk.push((key, value));
            self.blocks.push(blk);
            return (0, 0);
        }
        self.blocks[b].insert(i, (key, value));
        let n = self.blocks[b].len();
        if n > BLOCK_MAX {
            let half = n / 2;
            let tail = self.blocks[b].split_off(half);
            self.blocks.insert(b + 1, tail);
            if i >= half {
                return (b + 1, i - half);
            }
        }
        (b, i)
    }

    /// Number of entries between positions p and q (p <= q).
    fn count_between(&self, p: (usize, usize), q: (usize, usize)) -> usize {
        if p.0 == q.0 {
            return q.1 - p.1;
        }
        let mut n = self.blocks[p.0].len() - p.1;
        let mut b = p.0 + 1;
        while b < q.0 {
            n += self.blocks[b].len();
            b += 1;
        }
        if q.0 < self.blocks.len() {
            n += q.1;
        }
        n
    }

    /// All entries in order, consuming the map.
    fn into_flat(self) -> Vec<(K, V)> {
        let mut out = Vec::with_capacity(self.len);
        let mut bs = self.blocks.into_iter();
        while let Some(b) = bs.next() {
            out.extend(b);
        }
        out
    }

    /// Rebuilds a map from sorted, deduplicated entries.
    fn from_sorted(v: Vec<(K, V)>) -> BTreeMap<K, V> {
        let len = v.len();
        let mut blocks = Vec::new();
        let mut it = v.into_iter();
        let mut cur: Vec<(K, V)> = Vec::new();
        while let Some(e) = it.next() {
            cur.push(e);
            if cur.len() == BLOCK_MAX / 2 {
                blocks.push(mem::take(&mut cur));
            }
        }
        if !cur.is_empty() {
            blocks.push(cur);
        }
        BTreeMap { blocks, len }
    }
}

impl<K: Ord, V> BTreeMap<K, V> {
    /// Binary search for `key`: Ok(position) if present, Err(insertion point)
    /// otherwise. Positions are (block, index); the insertion point past the
    /// last entry is (last block, its length).
    #[inline]
    fn search<Q: ?Sized + Ord>(&self, key: &Q) -> Result<(usize, usize), (usize, usize)>
    where
        K: Borrow<Q>,
    {
        let nb = self.blocks.len();
        if nb == 0 {
            return Err((0, 0));
        }
        // the first block whose last key is >= key
        let mut lo = 0usize;
        let mut hi = nb;
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let blk = &self.blocks[mid];
            if blk[blk.len() - 1].0.borrow() < key {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        if lo == nb {
            return Err((nb - 1, self.blocks[nb - 1].len()));
        }
        let blk = &self.blocks[lo];
        let mut l = 0usize;
        let mut h = blk.len();
        while l < h {
            let mid = l + (h - l) / 2;
            match blk[mid].0.borrow().cmp(key) {
                Ordering::Less => l = mid + 1,
                Ordering::Greater => h = mid,
                Ordering::Equal => return Ok((lo, mid)),
            }
        }
        Err((lo, l))
    }

    /// Returns a reference to the value corresponding to the key.
    #[inline]
    pub fn get<Q: ?Sized>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Ord,
    {
        match self.search(key) {
            Ok((b, i)) => Some(&self.blocks[b][i].1),
            Err(_) => None,
        }
    }

    /// Returns the key-value pair corresponding to the supplied key.
    #[inline]
    pub fn get_key_value<Q: ?Sized>(&self, k: &Q) -> Option<(&K, &V)>
    where
        K: Borrow<Q>,
        Q: Ord,
    {
        match self.search(k) {
            Ok((b, i)) => {
                let e = &self.blocks[b][i];
                Some((&e.0, &e.1))
            }
            Err(_) => None,
        }
    }

    /// Returns `true` if the map contains a value for the specified key.
    #[inline]
    pub fn contains_key<Q: ?Sized>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Ord,
    {
        self.search(key).is_ok()
    }

    /// Returns a mutable reference to the value corresponding to the key.
    #[inline]
    pub fn get_mut<Q: ?Sized>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Ord,
    {
        match self.search(key) {
            Ok((b, i)) => Some(&mut self.blocks[b][i].1),
            Err(_) => None,
        }
    }

    /// Inserts a key-value pair into the map, returning the old value if the key was present.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        match self.search(&key) {
            Ok((b, i)) => Some(mem::replace(&mut self.blocks[b][i].1, value)),
            Err((b, i)) => {
                self.insert_at(b, i, key, value);
                None
            }
        }
    }

    /// Removes a key from the map, returning the value at the key if the key was previously in the map.
    #[inline]
    pub fn remove<Q: ?Sized>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Ord,
    {
        match self.search(key) {
            Ok((b, i)) => Some(self.remove_at(b, i).1),
            Err(_) => None,
        }
    }

    /// Removes a key from the map, returning the stored key and value if the key was previously in the map.
    #[inline]
    pub fn remove_entry<Q: ?Sized>(&mut self, key: &Q) -> Option<(K, V)>
    where
        K: Borrow<Q>,
        Q: Ord,
    {
        match self.search(key) {
            Ok((b, i)) => Some(self.remove_at(b, i)),
            Err(_) => None,
        }
    }

    /// Moves all elements from `other` into `self`, leaving `other` empty.
    pub fn append(&mut self, other: &mut BTreeMap<K, V>) {
        let items = mem::take(other).into_flat();
        let mut it = items.into_iter();
        while let Some((k, v)) = it.next() {
            self.insert(k, v);
        }
    }

    /// The position of the first entry not below the start bound.
    fn lower_pos<T: ?Sized + Ord>(&self, b: Bound<&T>) -> (usize, usize)
    where
        K: Borrow<T>,
    {
        let p = match b {
            Bound::Included(s) => match self.search(s) {
                Ok(p) => p,
                Err(p) => p,
            },
            Bound::Excluded(s) => match self.search(s) {
                Ok((b, i)) => (b, i + 1),
                Err(p) => p,
            },
            Bound::Unbounded => (0, 0),
        };
        self.norm(p)
    }

    /// The position just past the last entry not above the end bound.
    fn upper_pos<T: ?Sized + Ord>(&self, b: Bound<&T>) -> (usize, usize)
    where
        K: Borrow<T>,
    {
        let p = match b {
            Bound::Included(e) => match self.search(e) {
                Ok((b, i)) => (b, i + 1),
                Err(p) => p,
            },
            Bound::Excluded(e) => match self.search(e) {
                Ok(p) => p,
                Err(p) => p,
            },
            Bound::Unbounded => (self.blocks.len(), 0),
        };
        self.norm(p)
    }

    /// Normalizes a position at the end of a block to the start of the next.
    fn norm(&self, p: (usize, usize)) -> (usize, usize) {
        if p.0 < self.blocks.len() && p.1 >= self.blocks[p.0].len() {
            (p.0 + 1, 0)
        } else {
            p
        }
    }

    fn range_positions<T: ?Sized, R>(&self, range: &R) -> ((usize, usize), (usize, usize), usize)
    where
        T: Ord,
        K: Borrow<T>,
        R: RangeBounds<T>,
    {
        let sb = match range.start_bound() {
            Bound::Included(s) => Some((s, false)),
            Bound::Excluded(s) => Some((s, true)),
            Bound::Unbounded => None,
        };
        let eb = match range.end_bound() {
            Bound::Included(e) => Some((e, false)),
            Bound::Excluded(e) => Some((e, true)),
            Bound::Unbounded => None,
        };
        // (std only validates the bounds when the map has a root node)
        if let (Some((s, s_excl)), Some((e, e_excl)), true) = (sb, eb, self.len != 0) {
            if s > e {
                crate::rt::panic_str("range start is greater than range end in BTreeMap", file!(), line!(), column!());
            }
            if s_excl && e_excl && s == e {
                crate::rt::panic_str("range start and end are equal and excluded in BTreeMap", file!(), line!(), column!());
            }
        }
        let start = self.lower_pos(range.start_bound());
        let end = self.upper_pos(range.end_bound());
        if end <= start {
            return (start, start, 0);
        }
        let n = self.count_between(start, end);
        (start, end, n)
    }

    /// Constructs a double-ended iterator over a sub-range of elements in the map.
    pub fn range<T: ?Sized, R>(&self, range: R) -> Range<'_, K, V>
    where
        T: Ord,
        K: Borrow<T> + Ord,
        R: RangeBounds<T>,
    {
        let (start, end, n) = self.range_positions(&range);
        Range { inner: Flat::between(&self.blocks, start, end, n) }
    }

    /// Constructs a mutable double-ended iterator over a sub-range of elements in the map.
    pub fn range_mut<T: ?Sized, R>(&mut self, range: R) -> RangeMut<'_, K, V>
    where
        T: Ord,
        K: Borrow<T> + Ord,
        R: RangeBounds<T>,
    {
        let (start, end, n) = self.range_positions(&range);
        RangeMut { inner: FlatMut::between(&mut self.blocks, start, end, n) }
    }

    /// Gets the given key's corresponding entry in the map for in-place manipulation.
    pub fn entry(&mut self, key: K) -> Entry<'_, K, V> {
        match self.search(&key) {
            Ok((b, i)) => Entry::Occupied(OccupiedEntry { map: self, b, i }),
            Err((b, i)) => Entry::Vacant(VacantEntry { map: self, b, i, key }),
        }
    }

    /// Splits the collection into two at the given key. Returns everything after the given key, including the key.
    pub fn split_off<Q: ?Sized + Ord>(&mut self, key: &Q) -> Self
    where
        K: Borrow<Q>,
    {
        let (b, i) = self.norm(match self.search(key) {
            Ok(p) => p,
            Err(p) => p,
        });
        if b >= self.blocks.len() {
            return BTreeMap::new();
        }
        let mut tail_blocks = self.blocks.split_off(b + 1);
        let tail_of_b = self.blocks[b].split_off(i);
        if self.blocks[b].is_empty() {
            self.blocks.pop();
        }
        if !tail_of_b.is_empty() {
            tail_blocks.insert(0, tail_of_b);
        }
        let mut moved = 0;
        let mut k = 0;
        while k < tail_blocks.len() {
            moved += tail_blocks[k].len();
            k += 1;
        }
        self.len -= moved;
        BTreeMap { blocks: tail_blocks, len: moved }
    }

    /// Creates an iterator that visits all elements in ascending key order and uses a closure to determine if an element should be removed.
    ///
    /// NOTE: rustgo extracts eagerly (all matching elements in `range` are
    /// removed even if the iterator is not exhausted).
    pub fn extract_if<F, R>(&mut self, range: R, pred: F) -> vec::IntoIter<(K, V)>
    where
        K: Ord,
        R: RangeBounds<K>,
        F: FnMut(&K, &mut V) -> bool,
    {
        let mut pred = pred;
        let mut removed = Vec::new();
        let mut kept = Vec::with_capacity(self.len);
        let mut it = mem::take(self).into_flat().into_iter();
        while let Some(mut e) = it.next() {
            if range.contains(&e.0) && pred(&e.0, &mut e.1) {
                removed.push(e);
            } else {
                kept.push(e);
            }
        }
        *self = BTreeMap::from_sorted(kept);
        removed.into_iter()
    }
}

impl<K: Clone, V: Clone> Clone for BTreeMap<K, V> {
    fn clone(&self) -> BTreeMap<K, V> {
        BTreeMap { blocks: self.blocks.clone(), len: self.len }
    }
}

impl<K, V> Default for BTreeMap<K, V> {
    fn default() -> BTreeMap<K, V> {
        BTreeMap::new()
    }
}

impl<K: PartialEq, V: PartialEq> PartialEq for BTreeMap<K, V> {
    fn eq(&self, other: &BTreeMap<K, V>) -> bool {
        if self.len != other.len {
            return false;
        }
        let mut a = self.iter();
        let mut b = other.iter();
        while let Some((ka, va)) = a.next() {
            match b.next() {
                Some((kb, vb)) => {
                    if ka != kb || va != vb {
                        return false;
                    }
                }
                None => return false,
            }
        }
        true
    }
}

impl<K: Eq, V: Eq> Eq for BTreeMap<K, V> {}

impl<K: PartialOrd, V: PartialOrd> PartialOrd for BTreeMap<K, V> {
    #[inline]
    fn partial_cmp(&self, other: &BTreeMap<K, V>) -> Option<Ordering> {
        self.iter().partial_cmp(other.iter())
    }
}

impl<K: Ord, V: Ord> Ord for BTreeMap<K, V> {
    #[inline]
    fn cmp(&self, other: &BTreeMap<K, V>) -> Ordering {
        self.iter().cmp(other.iter())
    }
}

impl<K: Hash, V: Hash> Hash for BTreeMap<K, V> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_length_prefix(self.len());
        let mut it = self.iter();
        while let Some(elt) = it.next() {
            elt.hash(state);
        }
    }
}

impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for BTreeMap<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl<K, Q: ?Sized, V> Index<&Q> for BTreeMap<K, V>
where
    K: Borrow<Q> + Ord,
    Q: Ord,
{
    type Output = V;
    #[inline]
    #[track_caller]
    fn index(&self, key: &Q) -> &V {
        match self.get(key) {
            Some(v) => v,
            None => crate::rt::panic_str("no entry found for key", file!(), line!(), column!()),
        }
    }
}

impl<K: Ord, V> FromIterator<(K, V)> for BTreeMap<K, V> {
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> BTreeMap<K, V> {
        let mut map = BTreeMap::new();
        map.extend(iter);
        map
    }
}

impl<K: Ord, V> Extend<(K, V)> for BTreeMap<K, V> {
    #[inline]
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) {
        let mut it = iter.into_iter();
        while let Some((k, v)) = it.next() {
            self.insert(k, v);
        }
    }
    #[inline]
    fn extend_one(&mut self, (k, v): (K, V)) {
        self.insert(k, v);
    }
}

impl<'a, K: Ord + Copy, V: Copy> Extend<(&'a K, &'a V)> for BTreeMap<K, V> {
    fn extend<I: IntoIterator<Item = (&'a K, &'a V)>>(&mut self, iter: I) {
        let mut it = iter.into_iter();
        while let Some((k, v)) = it.next() {
            self.insert(*k, *v);
        }
    }
}

impl<K: Ord, V, const N: usize> From<[(K, V); N]> for BTreeMap<K, V> {
    fn from(arr: [(K, V); N]) -> Self {
        let mut map = BTreeMap::new();
        map.extend(arr);
        map
    }
}


impl<'a, K, V> IntoIterator for &'a BTreeMap<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;
    fn into_iter(self) -> Iter<'a, K, V> {
        self.iter()
    }
}

impl<'a, K, V> IntoIterator for &'a mut BTreeMap<K, V> {
    type Item = (&'a K, &'a mut V);
    type IntoIter = IterMut<'a, K, V>;
    fn into_iter(self) -> IterMut<'a, K, V> {
        self.iter_mut()
    }
}

impl<K, V> IntoIterator for BTreeMap<K, V> {
    type Item = (K, V);
    type IntoIter = IntoIter<K, V>;
    fn into_iter(self) -> IntoIter<K, V> {
        IntoIter { inner: self.into_flat().into_iter() }
    }
}

// ---------------------------------------------------------------------------
// Iterators

/// The entries of a run of blocks: the rest of the front block, the blocks
/// in between, the start of the back block; `n` entries remain.
struct Flat<'a, K: 'a, V: 'a> {
    front: slice::Iter<'a, (K, V)>,
    mid: slice::Iter<'a, Vec<(K, V)>>,
    back: slice::Iter<'a, (K, V)>,
    n: usize,
}

impl<'a, K, V> Clone for Flat<'a, K, V> {
    fn clone(&self) -> Self {
        Flat { front: self.front.clone(), mid: self.mid.clone(), back: self.back.clone(), n: self.n }
    }
}

impl<'a, K, V> Flat<'a, K, V> {
    fn new(blocks: &'a [Vec<(K, V)>], n: usize) -> Self {
        Flat { front: Default::default(), mid: blocks.iter(), back: Default::default(), n }
    }

    fn between(blocks: &'a [Vec<(K, V)>], s: (usize, usize), e: (usize, usize), n: usize) -> Self {
        if n == 0 {
            return Flat { front: Default::default(), mid: Default::default(), back: Default::default(), n: 0 };
        }
        if s.0 == e.0 {
            return Flat { front: blocks[s.0][s.1..e.1].iter(), mid: Default::default(), back: Default::default(), n };
        }
        let back = if e.0 < blocks.len() { blocks[e.0][..e.1].iter() } else { Default::default() };
        Flat { front: blocks[s.0][s.1..].iter(), mid: blocks[s.0 + 1..e.0].iter(), back, n }
    }

    #[inline]
    fn next(&mut self) -> Option<&'a (K, V)> {
        if self.n == 0 {
            return None;
        }
        loop {
            if let Some(e) = self.front.next() {
                self.n -= 1;
                return Some(e);
            }
            match self.mid.next() {
                Some(b) => self.front = b.iter(),
                None => {
                    let e = self.back.next();
                    if e.is_some() {
                        self.n -= 1;
                    }
                    return e;
                }
            }
        }
    }

    #[inline]
    fn next_back(&mut self) -> Option<&'a (K, V)> {
        if self.n == 0 {
            return None;
        }
        loop {
            if let Some(e) = self.back.next_back() {
                self.n -= 1;
                return Some(e);
            }
            match self.mid.next_back() {
                Some(b) => self.back = b.iter(),
                None => {
                    let e = self.front.next_back();
                    if e.is_some() {
                        self.n -= 1;
                    }
                    return e;
                }
            }
        }
    }
}

/// Mutable version of `Flat`.
struct FlatMut<'a, K: 'a, V: 'a> {
    front: slice::IterMut<'a, (K, V)>,
    mid: slice::IterMut<'a, Vec<(K, V)>>,
    back: slice::IterMut<'a, (K, V)>,
    n: usize,
}

impl<'a, K, V> FlatMut<'a, K, V> {
    fn new(blocks: &'a mut [Vec<(K, V)>], n: usize) -> Self {
        FlatMut { front: Default::default(), mid: blocks.iter_mut(), back: Default::default(), n }
    }

    fn between(blocks: &'a mut [Vec<(K, V)>], s: (usize, usize), e: (usize, usize), n: usize) -> Self {
        if n == 0 {
            return FlatMut { front: Default::default(), mid: Default::default(), back: Default::default(), n: 0 };
        }
        let (head, rest) = blocks.split_at_mut(s.0 + 1);
        let first: &'a mut Vec<(K, V)> = &mut head[s.0];
        if s.0 == e.0 {
            return FlatMut { front: first[s.1..e.1].iter_mut(), mid: Default::default(), back: Default::default(), n };
        }
        let (mid, tail) = rest.split_at_mut(e.0 - s.0 - 1);
        let back = match tail.first_mut() {
            Some(b) => b[..e.1].iter_mut(),
            None => Default::default(),
        };
        FlatMut { front: first[s.1..].iter_mut(), mid: mid.iter_mut(), back, n }
    }

    #[inline]
    fn next(&mut self) -> Option<&'a mut (K, V)> {
        if self.n == 0 {
            return None;
        }
        loop {
            if let Some(e) = self.front.next() {
                self.n -= 1;
                return Some(e);
            }
            match self.mid.next() {
                Some(b) => self.front = b.iter_mut(),
                None => {
                    let e = self.back.next();
                    if e.is_some() {
                        self.n -= 1;
                    }
                    return e;
                }
            }
        }
    }

    #[inline]
    fn next_back(&mut self) -> Option<&'a mut (K, V)> {
        if self.n == 0 {
            return None;
        }
        loop {
            if let Some(e) = self.back.next_back() {
                self.n -= 1;
                return Some(e);
            }
            match self.mid.next_back() {
                Some(b) => self.back = b.iter_mut(),
                None => {
                    let e = self.front.next_back();
                    if e.is_some() {
                        self.n -= 1;
                    }
                    return e;
                }
            }
        }
    }
}

macro_rules! __std_btree_ref_iter {
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        pub struct $name<'a, K: 'a, V: 'a> {
            inner: Flat<'a, K, V>,
        }

        impl<'a, K, V> Clone for $name<'a, K, V> {
            fn clone(&self) -> Self {
                $name { inner: self.inner.clone() }
            }
        }

        impl<'a, K, V> Iterator for $name<'a, K, V> {
            type Item = (&'a K, &'a V);
            #[inline]
            fn next(&mut self) -> Option<(&'a K, &'a V)> {
                match self.inner.next() {
                    Some(e) => Some((&e.0, &e.1)),
                    None => None,
                }
            }
            #[inline]
            fn size_hint(&self) -> (usize, Option<usize>) {
                (self.inner.n, Some(self.inner.n))
            }
            #[inline]
            fn last(self) -> Option<(&'a K, &'a V)> {
                let mut s = self;
                s.next_back()
            }
        }

        impl<'a, K, V> DoubleEndedIterator for $name<'a, K, V> {
            #[inline]
            fn next_back(&mut self) -> Option<(&'a K, &'a V)> {
                match self.inner.next_back() {
                    Some(e) => Some((&e.0, &e.1)),
                    None => None,
                }
            }
        }

        impl<'a, K, V> ExactSizeIterator for $name<'a, K, V> {
            fn len(&self) -> usize {
                self.inner.n
            }
        }

        impl<'a, K, V> FusedIterator for $name<'a, K, V> {}

        impl<'a, K: fmt::Debug, V: fmt::Debug> fmt::Debug for $name<'a, K, V> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_list().entries(self.clone()).finish()
            }
        }
    };
}

__std_btree_ref_iter!(Iter, "An iterator over the entries of a `BTreeMap`.");
__std_btree_ref_iter!(Range, "An iterator over a sub-range of entries in a `BTreeMap`.");

macro_rules! __std_btree_mut_iter {
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        pub struct $name<'a, K: 'a, V: 'a> {
            inner: FlatMut<'a, K, V>,
        }

        impl<'a, K, V> Iterator for $name<'a, K, V> {
            type Item = (&'a K, &'a mut V);
            #[inline]
            fn next(&mut self) -> Option<(&'a K, &'a mut V)> {
                match self.inner.next() {
                    Some(e) => Some((&e.0, &mut e.1)),
                    None => None,
                }
            }
            #[inline]
            fn size_hint(&self) -> (usize, Option<usize>) {
                (self.inner.n, Some(self.inner.n))
            }
        }

        impl<'a, K, V> DoubleEndedIterator for $name<'a, K, V> {
            #[inline]
            fn next_back(&mut self) -> Option<(&'a K, &'a mut V)> {
                match self.inner.next_back() {
                    Some(e) => Some((&e.0, &mut e.1)),
                    None => None,
                }
            }
        }

        impl<'a, K, V> ExactSizeIterator for $name<'a, K, V> {
            fn len(&self) -> usize {
                self.inner.n
            }
        }

        impl<'a, K, V> FusedIterator for $name<'a, K, V> {}
    };
}

__std_btree_mut_iter!(IterMut, "A mutable iterator over the entries of a `BTreeMap`.");
__std_btree_mut_iter!(RangeMut, "A mutable iterator over a sub-range of entries in a `BTreeMap`.");

/// An owning iterator over the entries of a `BTreeMap`, sorted by key.
pub struct IntoIter<K, V> {
    inner: vec::IntoIter<(K, V)>,
}

impl<K, V> Iterator for IntoIter<K, V> {
    type Item = (K, V);
    #[inline]
    fn next(&mut self) -> Option<(K, V)> {
        self.inner.next()
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<K, V> DoubleEndedIterator for IntoIter<K, V> {
    #[inline]
    fn next_back(&mut self) -> Option<(K, V)> {
        self.inner.next_back()
    }
}

impl<K, V> ExactSizeIterator for IntoIter<K, V> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<K, V> FusedIterator for IntoIter<K, V> {}

/// An iterator over the keys of a `BTreeMap`.
pub struct Keys<'a, K: 'a, V: 'a> {
    inner: Iter<'a, K, V>,
}

impl<'a, K, V> Clone for Keys<'a, K, V> {
    fn clone(&self) -> Self {
        Keys { inner: self.inner.clone() }
    }
}

impl<'a, K, V> Iterator for Keys<'a, K, V> {
    type Item = &'a K;
    #[inline]
    fn next(&mut self) -> Option<&'a K> {
        match self.inner.next() {
            Some((k, _)) => Some(k),
            None => None,
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
    #[inline]
    fn last(self) -> Option<&'a K> {
        let mut s = self;
        s.next_back()
    }
}

impl<'a, K, V> DoubleEndedIterator for Keys<'a, K, V> {
    #[inline]
    fn next_back(&mut self) -> Option<&'a K> {
        match self.inner.next_back() {
            Some((k, _)) => Some(k),
            None => None,
        }
    }
}

impl<'a, K, V> ExactSizeIterator for Keys<'a, K, V> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<'a, K, V> FusedIterator for Keys<'a, K, V> {}

impl<'a, K: fmt::Debug, V> fmt::Debug for Keys<'a, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

/// An iterator over the values of a `BTreeMap`.
pub struct Values<'a, K: 'a, V: 'a> {
    inner: Iter<'a, K, V>,
}

impl<'a, K, V> Clone for Values<'a, K, V> {
    fn clone(&self) -> Self {
        Values { inner: self.inner.clone() }
    }
}

impl<'a, K, V> Iterator for Values<'a, K, V> {
    type Item = &'a V;
    #[inline]
    fn next(&mut self) -> Option<&'a V> {
        match self.inner.next() {
            Some((_, v)) => Some(v),
            None => None,
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
    #[inline]
    fn last(self) -> Option<&'a V> {
        let mut s = self;
        s.next_back()
    }
}

impl<'a, K, V> DoubleEndedIterator for Values<'a, K, V> {
    #[inline]
    fn next_back(&mut self) -> Option<&'a V> {
        match self.inner.next_back() {
            Some((_, v)) => Some(v),
            None => None,
        }
    }
}

impl<'a, K, V> ExactSizeIterator for Values<'a, K, V> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<'a, K, V> FusedIterator for Values<'a, K, V> {}

impl<'a, K, V: fmt::Debug> fmt::Debug for Values<'a, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

/// A mutable iterator over the values of a `BTreeMap`.
pub struct ValuesMut<'a, K: 'a, V: 'a> {
    inner: IterMut<'a, K, V>,
}

impl<'a, K, V> Iterator for ValuesMut<'a, K, V> {
    type Item = &'a mut V;
    #[inline]
    fn next(&mut self) -> Option<&'a mut V> {
        match self.inner.next() {
            Some((_, v)) => Some(v),
            None => None,
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a, K, V> DoubleEndedIterator for ValuesMut<'a, K, V> {
    #[inline]
    fn next_back(&mut self) -> Option<&'a mut V> {
        match self.inner.next_back() {
            Some((_, v)) => Some(v),
            None => None,
        }
    }
}

impl<'a, K, V> ExactSizeIterator for ValuesMut<'a, K, V> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

/// An owning iterator over the keys of a `BTreeMap`.
pub struct IntoKeys<K, V> {
    inner: IntoIter<K, V>,
}

impl<K, V> Iterator for IntoKeys<K, V> {
    type Item = K;
    #[inline]
    fn next(&mut self) -> Option<K> {
        match self.inner.next() {
            Some((k, _)) => Some(k),
            None => None,
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<K, V> DoubleEndedIterator for IntoKeys<K, V> {
    #[inline]
    fn next_back(&mut self) -> Option<K> {
        match self.inner.next_back() {
            Some((k, _)) => Some(k),
            None => None,
        }
    }
}

/// An owning iterator over the values of a `BTreeMap`.
pub struct IntoValues<K, V> {
    inner: IntoIter<K, V>,
}

impl<K, V> Iterator for IntoValues<K, V> {
    type Item = V;
    #[inline]
    fn next(&mut self) -> Option<V> {
        match self.inner.next() {
            Some((_, v)) => Some(v),
            None => None,
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<K, V> DoubleEndedIterator for IntoValues<K, V> {
    #[inline]
    fn next_back(&mut self) -> Option<V> {
        match self.inner.next_back() {
            Some((_, v)) => Some(v),
            None => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Entry API

/// A view into a single entry in a map, which may either be vacant or occupied.
pub enum Entry<'a, K: 'a, V: 'a> {
    /// A vacant entry.
    Vacant(VacantEntry<'a, K, V>),
    /// An occupied entry.
    Occupied(OccupiedEntry<'a, K, V>),
}

pub use self::Entry::{Occupied, Vacant};

/// A view into a vacant entry in a `BTreeMap`.
pub struct VacantEntry<'a, K: 'a, V: 'a> {
    map: &'a mut BTreeMap<K, V>,
    b: usize,
    i: usize,
    key: K,
}

/// A view into an occupied entry in a `BTreeMap`.
pub struct OccupiedEntry<'a, K: 'a, V: 'a> {
    map: &'a mut BTreeMap<K, V>,
    b: usize,
    i: usize,
}

impl<'a, K: Ord, V> Entry<'a, K, V> {
    /// Ensures a value is in the entry by inserting the default if empty, and returns a mutable reference to the value in the entry.
    pub fn or_insert(self, default: V) -> &'a mut V {
        match self {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(default),
        }
    }

    /// Ensures a value is in the entry by inserting the result of the default function if empty.
    pub fn or_insert_with<F: FnOnce() -> V>(self, default: F) -> &'a mut V {
        match self {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(default()),
        }
    }

    /// Ensures a value is in the entry by inserting, if empty, the result of the default function called with the key.
    pub fn or_insert_with_key<F: FnOnce(&K) -> V>(self, default: F) -> &'a mut V {
        match self {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let value = default(entry.key());
                entry.insert(value)
            }
        }
    }

    /// Returns a reference to this entry's key.
    pub fn key(&self) -> &K {
        match *self {
            Entry::Occupied(ref entry) => entry.key(),
            Entry::Vacant(ref entry) => entry.key(),
        }
    }

    /// Provides in-place mutable access to an occupied entry before any potential inserts into the map.
    pub fn and_modify<F>(self, f: F) -> Self
    where
        F: FnOnce(&mut V),
    {
        match self {
            Entry::Occupied(mut entry) => {
                f(entry.get_mut());
                Entry::Occupied(entry)
            }
            Entry::Vacant(entry) => Entry::Vacant(entry),
        }
    }

    /// Sets the value of the entry, and returns an `OccupiedEntry`.
    pub fn insert_entry(self, value: V) -> OccupiedEntry<'a, K, V> {
        match self {
            Entry::Occupied(mut entry) => {
                entry.insert(value);
                entry
            }
            Entry::Vacant(entry) => entry.insert_entry(value),
        }
    }
}

impl<'a, K: Ord, V: Default> Entry<'a, K, V> {
    /// Ensures a value is in the entry by inserting the default value if empty.
    pub fn or_default(self) -> &'a mut V {
        match self {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(Default::default()),
        }
    }
}

impl<'a, K: Ord, V> VacantEntry<'a, K, V> {
    /// Gets a reference to the key that would be used when inserting a value through the `VacantEntry`.
    pub fn key(&self) -> &K {
        &self.key
    }

    /// Take ownership of the key.
    pub fn into_key(self) -> K {
        self.key
    }

    /// Sets the value of the entry with the `VacantEntry`'s key, and returns a mutable reference to it.
    pub fn insert(self, value: V) -> &'a mut V {
        let map = self.map;
        let (b, i) = map.insert_at(self.b, self.i, self.key, value);
        &mut map.blocks[b][i].1
    }

    /// Sets the value of the entry with the `VacantEntry`'s key, and returns an `OccupiedEntry`.
    pub fn insert_entry(self, value: V) -> OccupiedEntry<'a, K, V> {
        let map = self.map;
        let (b, i) = map.insert_at(self.b, self.i, self.key, value);
        OccupiedEntry { map, b, i }
    }
}

impl<'a, K: Ord, V> OccupiedEntry<'a, K, V> {
    /// Gets a reference to the key in the entry.
    pub fn key(&self) -> &K {
        &self.map.blocks[self.b][self.i].0
    }

    /// Take ownership of the key and value from the map.
    pub fn remove_entry(self) -> (K, V) {
        self.map.remove_at(self.b, self.i)
    }

    /// Gets a reference to the value in the entry.
    pub fn get(&self) -> &V {
        &self.map.blocks[self.b][self.i].1
    }

    /// Gets a mutable reference to the value in the entry.
    pub fn get_mut(&mut self) -> &mut V {
        &mut self.map.blocks[self.b][self.i].1
    }

    /// Converts the entry into a mutable reference to its value.
    pub fn into_mut(self) -> &'a mut V {
        let (b, i) = (self.b, self.i);
        &mut self.map.blocks[b][i].1
    }

    /// Sets the value of the entry with the `OccupiedEntry`'s key, and returns the entry's old value.
    pub fn insert(&mut self, value: V) -> V {
        mem::replace(self.get_mut(), value)
    }

    /// Takes the value of the entry out of the map, and returns it.
    pub fn remove(self) -> V {
        self.remove_entry().1
    }
}

impl<'a, K: fmt::Debug + Ord, V: fmt::Debug> fmt::Debug for Entry<'a, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Entry::Vacant(ref v) => f.debug_tuple("Entry").field(v).finish(),
            Entry::Occupied(ref o) => f.debug_tuple("Entry").field(o).finish(),
        }
    }
}

impl<'a, K: fmt::Debug + Ord, V> fmt::Debug for VacantEntry<'a, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("VacantEntry").field(self.key()).finish()
    }
}

impl<'a, K: fmt::Debug + Ord, V: fmt::Debug> fmt::Debug for OccupiedEntry<'a, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OccupiedEntry").field("key", self.key()).field("value", self.get()).finish()
    }
}

// ---------------------------------------------------------------------------
// Debug for the remaining iterators (lists of what is left, like std)

impl<'a, K, V> FlatMut<'a, K, V> {
    /// The remaining entries, in order.
    fn remaining(&self) -> impl Iterator<Item = &(K, V)> + '_ {
        let mid = self.mid.as_slice().iter().flat_map(|b| b.iter());
        self.front.as_slice().iter().chain(mid).chain(self.back.as_slice().iter()).take(self.n)
    }
}

impl<'a, K: fmt::Debug, V: fmt::Debug> fmt::Debug for IterMut<'a, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.inner.remaining().map(|e| (&e.0, &e.1))).finish()
    }
}

impl<'a, K: fmt::Debug, V: fmt::Debug> fmt::Debug for RangeMut<'a, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.inner.remaining().map(|e| (&e.0, &e.1))).finish()
    }
}

impl<'a, K, V: fmt::Debug> fmt::Debug for ValuesMut<'a, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.inner.inner.remaining().map(|e| &e.1)).finish()
    }
}

impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for IntoIter<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.inner.as_slice().iter().map(|e| (&e.0, &e.1))).finish()
    }
}

impl<K, V> IntoKeys<K, V> {
    /// The remaining keys (for `btree_set::IntoIter`'s Debug).
    pub(crate) fn remaining_keys(&self) -> impl Iterator<Item = &K> + '_ {
        self.inner.inner.as_slice().iter().map(|e| &e.0)
    }
}

impl<K: fmt::Debug, V> fmt::Debug for IntoKeys<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.inner.inner.as_slice().iter().map(|e| &e.0)).finish()
    }
}

impl<K, V: fmt::Debug> fmt::Debug for IntoValues<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.inner.inner.as_slice().iter().map(|e| &e.1)).finish()
    }
}

// ---------------------------------------------------------------------------
// Default for the iterators (empty iterators, like std)

impl<'a, K, V> Default for Iter<'a, K, V> {
    fn default() -> Self {
        Iter { inner: Flat { front: Default::default(), mid: Default::default(), back: Default::default(), n: 0 } }
    }
}

impl<'a, K, V> Default for Range<'a, K, V> {
    fn default() -> Self {
        Range { inner: Flat { front: Default::default(), mid: Default::default(), back: Default::default(), n: 0 } }
    }
}

impl<'a, K, V> Default for IterMut<'a, K, V> {
    fn default() -> Self {
        IterMut { inner: FlatMut { front: Default::default(), mid: Default::default(), back: Default::default(), n: 0 } }
    }
}

impl<'a, K, V> Default for Keys<'a, K, V> {
    fn default() -> Self {
        Keys { inner: Default::default() }
    }
}

impl<'a, K, V> Default for Values<'a, K, V> {
    fn default() -> Self {
        Values { inner: Default::default() }
    }
}

impl<K, V> Default for IntoIter<K, V> {
    fn default() -> Self {
        IntoIter { inner: Default::default() }
    }
}

impl<K, V> Default for IntoKeys<K, V> {
    fn default() -> Self {
        IntoKeys { inner: Default::default() }
    }
}

impl<K, V> Default for IntoValues<K, V> {
    fn default() -> Self {
        IntoValues { inner: Default::default() }
    }
}
