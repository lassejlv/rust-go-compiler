//! An ordered map (implemented as a sorted vector of entries).

use crate::borrow::Borrow;
use crate::cmp::Ordering;
use crate::fmt;
use crate::hash::{Hash, Hasher};
use crate::iter::{DoubleEndedIterator, ExactSizeIterator, Extend, FromIterator, FusedIterator, IntoIterator, Iterator};
use crate::mem;
use crate::ops::{Bound, Index, RangeBounds};
use crate::slice;
use crate::vec;

/// An ordered map based on a sorted vector.
pub struct BTreeMap<K, V> {
    entries: Vec<(K, V)>,
}

impl<K, V> BTreeMap<K, V> {
    /// Makes a new, empty `BTreeMap`.
    #[inline]
    pub const fn new() -> BTreeMap<K, V> {
        BTreeMap { entries: Vec::new() }
    }

    /// Clears the map, removing all elements.
    #[inline]
    pub fn clear(&mut self) {
        self.entries.clear()
    }

    /// Gets an iterator over the entries of the map, sorted by key.
    #[inline]
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter { inner: self.entries.iter() }
    }

    /// Gets a mutable iterator over the entries of the map, sorted by key.
    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        IterMut { inner: self.entries.iter_mut() }
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
        self.entries.len()
    }

    /// Returns `true` if the map contains no elements.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.entries.len() == 0
    }

    /// Returns the first key-value pair in the map (the minimum key).
    #[inline]
    pub fn first_key_value(&self) -> Option<(&K, &V)> {
        match self.entries.first() {
            Some(e) => Some((&e.0, &e.1)),
            None => None,
        }
    }

    /// Returns the last key-value pair in the map (the maximum key).
    #[inline]
    pub fn last_key_value(&self) -> Option<(&K, &V)> {
        match self.entries.last() {
            Some(e) => Some((&e.0, &e.1)),
            None => None,
        }
    }

    /// Removes and returns the first element in the map.
    #[inline]
    pub fn pop_first(&mut self) -> Option<(K, V)> {
        if self.entries.is_empty() {
            None
        } else {
            Some(self.entries.remove(0))
        }
    }

    /// Removes and returns the last element in the map.
    #[inline]
    pub fn pop_last(&mut self) -> Option<(K, V)> {
        self.entries.pop()
    }

    /// Returns the first entry in the map for in-place manipulation.
    pub fn first_entry(&mut self) -> Option<OccupiedEntry<'_, K, V>> {
        if self.entries.is_empty() {
            None
        } else {
            Some(OccupiedEntry { map: self, idx: 0 })
        }
    }

    /// Returns the last entry in the map for in-place manipulation.
    pub fn last_entry(&mut self) -> Option<OccupiedEntry<'_, K, V>> {
        let n = self.entries.len();
        if n == 0 {
            None
        } else {
            Some(OccupiedEntry { map: self, idx: n - 1 })
        }
    }

    /// Retains only the elements specified by the predicate.
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&K, &mut V) -> bool,
    {
        let mut f = f;
        self.entries.retain_mut(|e| f(&e.0, &mut e.1));
    }
}

impl<K: Ord, V> BTreeMap<K, V> {
    /// Binary search for `key`: Ok(index) if present, Err(insertion point) otherwise.
    #[inline]
    fn search<Q: ?Sized + Ord>(&self, key: &Q) -> Result<usize, usize>
    where
        K: Borrow<Q>,
    {
        let mut lo = 0usize;
        let mut hi = self.entries.len();
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            match self.entries[mid].0.borrow().cmp(key) {
                Ordering::Less => lo = mid + 1,
                Ordering::Greater => hi = mid,
                Ordering::Equal => return Ok(mid),
            }
        }
        Err(lo)
    }

    /// Returns a reference to the value corresponding to the key.
    #[inline]
    pub fn get<Q: ?Sized>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Ord,
    {
        match self.search(key) {
            Ok(i) => Some(&self.entries[i].1),
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
            Ok(i) => {
                let e = &self.entries[i];
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
            Ok(i) => Some(&mut self.entries[i].1),
            Err(_) => None,
        }
    }

    /// Inserts a key-value pair into the map, returning the old value if the key was present.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        match self.search(&key) {
            Ok(i) => Some(mem::replace(&mut self.entries[i].1, value)),
            Err(i) => {
                self.entries.insert(i, (key, value));
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
            Ok(i) => Some(self.entries.remove(i).1),
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
            Ok(i) => Some(self.entries.remove(i)),
            Err(_) => None,
        }
    }

    /// Moves all elements from `other` into `self`, leaving `other` empty.
    pub fn append(&mut self, other: &mut BTreeMap<K, V>) {
        let items = mem::take(&mut other.entries);
        let mut it = items.into_iter();
        while let Some((k, v)) = it.next() {
            self.insert(k, v);
        }
    }

    fn range_indices<T: ?Sized, R>(&self, range: &R) -> (usize, usize)
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
        if let (Some((s, s_excl)), Some((e, e_excl))) = (sb, eb) {
            if s > e {
                crate::rt::panic_str("range start is greater than range end in BTreeMap", file!(), line!(), column!());
            }
            if s_excl && e_excl && s == e {
                crate::rt::panic_str("range start and end are equal and excluded in BTreeMap", file!(), line!(), column!());
            }
        }
        let start = match range.start_bound() {
            Bound::Included(s) => match self.search(s) {
                Ok(i) => i,
                Err(i) => i,
            },
            Bound::Excluded(s) => match self.search(s) {
                Ok(i) => i + 1,
                Err(i) => i,
            },
            Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            Bound::Included(e) => match self.search(e) {
                Ok(i) => i + 1,
                Err(i) => i,
            },
            Bound::Excluded(e) => match self.search(e) {
                Ok(i) => i,
                Err(i) => i,
            },
            Bound::Unbounded => self.entries.len(),
        };
        let end = if end < start { start } else { end };
        (start, end)
    }

    /// Constructs a double-ended iterator over a sub-range of elements in the map.
    pub fn range<T: ?Sized, R>(&self, range: R) -> Range<'_, K, V>
    where
        T: Ord,
        K: Borrow<T> + Ord,
        R: RangeBounds<T>,
    {
        let (start, end) = self.range_indices(&range);
        Range { inner: self.entries[start..end].iter() }
    }

    /// Constructs a mutable double-ended iterator over a sub-range of elements in the map.
    pub fn range_mut<T: ?Sized, R>(&mut self, range: R) -> RangeMut<'_, K, V>
    where
        T: Ord,
        K: Borrow<T> + Ord,
        R: RangeBounds<T>,
    {
        let (start, end) = self.range_indices(&range);
        RangeMut { inner: self.entries[start..end].iter_mut() }
    }

    /// Gets the given key's corresponding entry in the map for in-place manipulation.
    pub fn entry(&mut self, key: K) -> Entry<'_, K, V> {
        match self.search(&key) {
            Ok(idx) => Entry::Occupied(OccupiedEntry { map: self, idx }),
            Err(idx) => Entry::Vacant(VacantEntry { map: self, idx, key }),
        }
    }

    /// Splits the collection into two at the given key. Returns everything after the given key, including the key.
    pub fn split_off<Q: ?Sized + Ord>(&mut self, key: &Q) -> Self
    where
        K: Borrow<Q>,
    {
        let idx = match self.search(key) {
            Ok(i) => i,
            Err(i) => i,
        };
        BTreeMap { entries: self.entries.split_off(idx) }
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
        let mut kept = Vec::with_capacity(self.entries.len());
        let mut it = mem::take(&mut self.entries).into_iter();
        while let Some(mut e) = it.next() {
            if range.contains(&e.0) && pred(&e.0, &mut e.1) {
                removed.push(e);
            } else {
                kept.push(e);
            }
        }
        self.entries = kept;
        removed.into_iter()
    }
}

impl<K: Clone, V: Clone> Clone for BTreeMap<K, V> {
    fn clone(&self) -> BTreeMap<K, V> {
        BTreeMap { entries: self.entries.clone() }
    }
}

impl<K, V> Default for BTreeMap<K, V> {
    fn default() -> BTreeMap<K, V> {
        BTreeMap::new()
    }
}

impl<K: PartialEq, V: PartialEq> PartialEq for BTreeMap<K, V> {
    fn eq(&self, other: &BTreeMap<K, V>) -> bool {
        self.entries == other.entries
    }
}

impl<K: Eq, V: Eq> Eq for BTreeMap<K, V> {}

impl<K: PartialOrd, V: PartialOrd> PartialOrd for BTreeMap<K, V> {
    #[inline]
    fn partial_cmp(&self, other: &BTreeMap<K, V>) -> Option<Ordering> {
        self.entries.partial_cmp(&other.entries)
    }
}

impl<K: Ord, V: Ord> Ord for BTreeMap<K, V> {
    #[inline]
    fn cmp(&self, other: &BTreeMap<K, V>) -> Ordering {
        self.entries.cmp(&other.entries)
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
        IntoIter { inner: self.entries.into_iter() }
    }
}

// ---------------------------------------------------------------------------
// Iterators

macro_rules! __std_btree_ref_iter {
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        pub struct $name<'a, K: 'a, V: 'a> {
            inner: slice::Iter<'a, (K, V)>,
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
                self.inner.size_hint()
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
                self.inner.len()
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
            inner: slice::IterMut<'a, (K, V)>,
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
                self.inner.size_hint()
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
                self.inner.len()
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
    idx: usize,
    key: K,
}

/// A view into an occupied entry in a `BTreeMap`.
pub struct OccupiedEntry<'a, K: 'a, V: 'a> {
    map: &'a mut BTreeMap<K, V>,
    idx: usize,
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
        let idx = self.idx;
        let map = self.map;
        map.entries.insert(idx, (self.key, value));
        &mut map.entries[idx].1
    }

    /// Sets the value of the entry with the `VacantEntry`'s key, and returns an `OccupiedEntry`.
    pub fn insert_entry(self, value: V) -> OccupiedEntry<'a, K, V> {
        let idx = self.idx;
        let map = self.map;
        map.entries.insert(idx, (self.key, value));
        OccupiedEntry { map, idx }
    }
}

impl<'a, K: Ord, V> OccupiedEntry<'a, K, V> {
    /// Gets a reference to the key in the entry.
    pub fn key(&self) -> &K {
        &self.map.entries[self.idx].0
    }

    /// Take ownership of the key and value from the map.
    pub fn remove_entry(self) -> (K, V) {
        self.map.entries.remove(self.idx)
    }

    /// Gets a reference to the value in the entry.
    pub fn get(&self) -> &V {
        &self.map.entries[self.idx].1
    }

    /// Gets a mutable reference to the value in the entry.
    pub fn get_mut(&mut self) -> &mut V {
        &mut self.map.entries[self.idx].1
    }

    /// Converts the entry into a mutable reference to its value.
    pub fn into_mut(self) -> &'a mut V {
        let idx = self.idx;
        &mut self.map.entries[idx].1
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
            Entry::Vacant(ref v) => f.debug_tuple("Entry").field(v.key()).finish(),
            Entry::Occupied(ref o) => f.debug_tuple("Entry").field(o.key()).finish(),
        }
    }
}
