//! A hash map implemented with linear probing over an insertion-ordered
//! entry vector (iteration order is unspecified, as in the real library).

use crate::borrow::Borrow;
use crate::fmt;
use crate::hash::{BuildHasher, Hash};

use crate::iter::{ExactSizeIterator, Extend, FromIterator, FusedIterator, IntoIterator, Iterator};
use crate::mem;
use crate::ops::Index;
use crate::slice;
use crate::vec;

pub use crate::hash::{DefaultHasher, RandomState};

const EMPTY: usize = usize::MAX;

#[derive(Clone)]
struct Bucket<K, V> {
    hash: u64,
    key: K,
    value: V,
}

/// The hash-independent storage of a `HashMap`.
#[derive(Clone)]
pub(crate) struct RawTable<K, V> {
    entries: Vec<Bucket<K, V>>,
    indices: Vec<usize>,
}

/// A hash map.
pub struct HashMap<K, V, S = RandomState> {
    table: RawTable<K, V>,
    hash_builder: S,
}

impl<K, V> RawTable<K, V> {
    #[inline]
    const fn new() -> RawTable<K, V> {
        RawTable { entries: Vec::new(), indices: Vec::new() }
    }

    fn with_capacity(capacity: usize) -> RawTable<K, V> {
        let mut m = RawTable { entries: Vec::with_capacity(capacity), indices: Vec::new() };
        if capacity > 0 {
            let mut n = 8;
            while n * 3 < capacity * 4 {
                n *= 2;
            }
            m.indices = vec::from_elem(EMPTY, n);
        }
        m
    }

    fn clear_indices(&mut self) {
        let mut i = 0;
        let n = self.indices.len();
        while i < n {
            self.indices[i] = EMPTY;
            i += 1;
        }
    }

    fn rebuild_indices(&mut self, table_len: usize) {
        let mut indices = vec::from_elem(EMPTY, table_len);
        if table_len > 0 {
            let mask = table_len - 1;
            let n = self.entries.len();
            let mut e = 0;
            while e < n {
                let mut i = (self.entries[e].hash as usize) & mask;
                while indices[i] != EMPTY {
                    i = (i + 1) & mask;
                }
                indices[i] = e;
                e += 1;
            }
        }
        self.indices = indices;
    }

    fn grow_if_needed(&mut self, additional: usize) {
        let needed = self.entries.len() + additional;
        let cap = self.indices.len();
        if needed * 4 > cap * 3 {
            let mut n = if cap == 0 { 8 } else { cap * 2 };
            while needed * 4 > n * 3 {
                n *= 2;
            }
            self.rebuild_indices(n);
        }
    }

    /// Removes the entry at `idx` (and its table `slot`), returning it.
    fn remove_at(&mut self, slot: usize, idx: usize) -> Bucket<K, V> {
        let mask = self.indices.len() - 1;
        // Backward-shift deletion.
        let mut i = slot;
        self.indices[i] = EMPTY;
        let mut j = i;
        loop {
            j = (j + 1) & mask;
            let e = self.indices[j];
            if e == EMPTY {
                break;
            }
            let ideal = (self.entries[e].hash as usize) & mask;
            let should_move = if i <= j { ideal <= i || ideal > j } else { ideal <= i && ideal > j };
            if should_move {
                self.indices[i] = e;
                self.indices[j] = EMPTY;
                i = j;
            }
        }
        let last = self.entries.len() - 1;
        let bucket = self.entries.swap_remove(idx);
        if idx != last {
            // The entry formerly at `last` now lives at `idx`: fix its slot.
            let mut s = (self.entries[idx].hash as usize) & mask;
            while self.indices[s] != last {
                s = (s + 1) & mask;
            }
            self.indices[s] = idx;
        }
        bucket
    }

    fn insert_new(&mut self, hash: u64, key: K, value: V) -> usize {
        self.grow_if_needed(1);
        let mask = self.indices.len() - 1;
        let mut i = (hash as usize) & mask;
        while self.indices[i] != EMPTY {
            i = (i + 1) & mask;
        }
        let idx = self.entries.len();
        self.indices[i] = idx;
        self.entries.push(Bucket { hash, key, value });
        idx
    }

    /// Finds the table slot and entry index for `key`.
    fn find<Q: ?Sized + Eq>(&self, hash: u64, key: &Q) -> (usize, Option<usize>)
    where
        K: Borrow<Q>,
    {
        let len = self.indices.len();
        if len == 0 {
            return (0, None);
        }
        let mask = len - 1;
        let mut i = (hash as usize) & mask;
        loop {
            let e = self.indices[i];
            if e == EMPTY {
                return (i, None);
            }
            let b = &self.entries[e];
            if b.hash == hash && b.key.borrow() == key {
                return (i, Some(e));
            }
            i = (i + 1) & mask;
        }
    }
}

impl<K, V> HashMap<K, V, RandomState> {
    /// Creates an empty `HashMap`.
    #[inline]
    pub fn new() -> HashMap<K, V, RandomState> {
        Self { table: RawTable::new(), hash_builder: RandomState::new() }
    }

    /// Creates an empty `HashMap` with at least the specified capacity.
    #[inline]
    pub fn with_capacity(capacity: usize) -> HashMap<K, V, RandomState> {
        Self { table: RawTable::with_capacity(capacity), hash_builder: RandomState::new() }
    }
}

impl<K, V, S> HashMap<K, V, S> {
    /// Creates an empty `HashMap` which will use the given hash builder to hash keys.
    #[inline]
    pub const fn with_hasher(hash_builder: S) -> HashMap<K, V, S> {
        Self { table: RawTable::new(), hash_builder }
    }

    /// Creates an empty `HashMap` with at least the specified capacity, using `hasher` to hash the keys.
    #[inline]
    pub fn with_capacity_and_hasher(capacity: usize, hasher: S) -> HashMap<K, V, S> {
        Self { table: RawTable::with_capacity(capacity), hash_builder: hasher }
    }

    /// Returns the number of elements the map can hold without reallocating.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.table.indices.len() * 3 / 4
    }

    /// An iterator visiting all keys in arbitrary order.
    #[inline]
    pub fn keys(&self) -> Keys<'_, K, V> {
        Keys { inner: self.iter() }
    }

    /// Creates a consuming iterator visiting all the keys in arbitrary order.
    #[inline]
    pub fn into_keys(self) -> IntoKeys<K, V> {
        IntoKeys { inner: self.into_iter() }
    }

    /// An iterator visiting all values in arbitrary order.
    #[inline]
    pub fn values(&self) -> Values<'_, K, V> {
        Values { inner: self.iter() }
    }

    /// An iterator visiting all values mutably in arbitrary order.
    #[inline]
    pub fn values_mut(&mut self) -> ValuesMut<'_, K, V> {
        ValuesMut { inner: self.iter_mut() }
    }

    /// Creates a consuming iterator visiting all the values in arbitrary order.
    #[inline]
    pub fn into_values(self) -> IntoValues<K, V> {
        IntoValues { inner: self.into_iter() }
    }

    /// An iterator visiting all key-value pairs in arbitrary order.
    #[inline]
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter { inner: self.table.entries.iter() }
    }

    /// An iterator visiting all key-value pairs in arbitrary order, with mutable references to the values.
    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        IterMut { inner: self.table.entries.iter_mut() }
    }

    /// Returns the number of elements in the map.
    #[inline]
    pub fn len(&self) -> usize {
        self.table.entries.len()
    }

    /// Returns `true` if the map contains no elements.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.table.entries.is_empty()
    }

    /// Clears the map, returning all key-value pairs as an iterator.
    #[inline]
    pub fn drain(&mut self) -> Drain<'_, K, V> {
        self.table.clear_indices();
        Drain { inner: self.table.entries.drain(..) }
    }

    /// Creates an iterator which uses a closure to determine if an element should be removed.
    pub fn extract_if<F>(&mut self, pred: F) -> vec::IntoIter<(K, V)>
    where
        F: FnMut(&K, &mut V) -> bool,
    {
        let mut pred = pred;
        let mut out = Vec::new();
        let mut kept = Vec::new();
        for mut b in mem::take(&mut self.table.entries) {
            if pred(&b.key, &mut b.value) {
                out.push((b.key, b.value));
            } else {
                kept.push(b);
            }
        }
        self.table.entries = kept;
        let n = self.table.indices.len();
        self.table.rebuild_indices(n);
        out.into_iter()
    }

    /// Retains only the elements specified by the predicate.
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&K, &mut V) -> bool,
    {
        let mut f = f;
        let before = self.table.entries.len();
        self.table.entries.retain_mut(|b| f(&b.key, &mut b.value));
        if self.table.entries.len() != before {
            let n = self.table.indices.len();
            self.table.rebuild_indices(n);
        }
    }

    /// Clears the map, removing all key-value pairs.
    #[inline]
    pub fn clear(&mut self) {
        self.table.entries.clear();
        self.table.clear_indices();
    }

    /// Returns a reference to the map's `BuildHasher`.
    #[inline]
    pub fn hasher(&self) -> &S {
        &self.hash_builder
    }
}

impl<K: Eq + Hash, V, S: BuildHasher> HashMap<K, V, S> {
    #[inline]
    fn make_hash<Q: Hash + ?Sized>(&self, q: &Q) -> u64 {
        self.hash_builder.hash_one(q)
    }

    /// Reserves capacity for at least `additional` more elements.
    pub fn reserve(&mut self, additional: usize) {
        self.table.entries.reserve(additional);
        self.table.grow_if_needed(additional);
    }

    /// Tries to reserve capacity for at least `additional` more elements.
    pub fn try_reserve(&mut self, additional: usize) -> Result<(), crate::collections::TryReserveError> {
        self.table.entries.try_reserve(additional)?;
        self.table.grow_if_needed(additional);
        Ok(())
    }

    /// Shrinks the capacity of the map as much as possible.
    pub fn shrink_to_fit(&mut self) {
        self.table.entries.shrink_to_fit();
    }

    /// Shrinks the capacity of the map with a lower limit.
    pub fn shrink_to(&mut self, min_capacity: usize) {
        self.table.entries.shrink_to(min_capacity);
    }

    /// Gets the given key's corresponding entry in the map for in-place manipulation.
    pub fn entry(&mut self, key: K) -> Entry<'_, K, V> {
        let hash = self.make_hash(&key);
        match self.table.find(hash, &key) {
            (_, Some(idx)) => Entry::Occupied(OccupiedEntry { map: &mut self.table, idx, key: Some(key) }),
            (_, None) => Entry::Vacant(VacantEntry { map: &mut self.table, hash, key }),
        }
    }

    /// Returns a reference to the value corresponding to the key.
    #[inline]
    pub fn get<Q: ?Sized>(&self, k: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        match self.table.find(self.make_hash(k), k) {
            (_, Some(idx)) => Some(&self.table.entries[idx].value),
            _ => None,
        }
    }

    /// Returns the key-value pair corresponding to the supplied key.
    #[inline]
    pub fn get_key_value<Q: ?Sized>(&self, k: &Q) -> Option<(&K, &V)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        match self.table.find(self.make_hash(k), k) {
            (_, Some(idx)) => {
                let b = &self.table.entries[idx];
                Some((&b.key, &b.value))
            }
            _ => None,
        }
    }

    /// Returns `true` if the map contains a value for the specified key.
    #[inline]
    pub fn contains_key<Q: ?Sized>(&self, k: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.table.find(self.make_hash(k), k).1.is_some()
    }

    /// Returns a mutable reference to the value corresponding to the key.
    #[inline]
    pub fn get_mut<Q: ?Sized>(&mut self, k: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        match self.table.find(self.make_hash(k), k) {
            (_, Some(idx)) => Some(&mut self.table.entries[idx].value),
            _ => None,
        }
    }

    /// Attempts to get mutable references to `N` values in the map at once.
    pub fn get_disjoint_mut<Q: ?Sized, const N: usize>(&mut self, ks: [&Q; N]) -> [Option<&mut V>; N]
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        let mut idxs = [usize::MAX; N];
        let mut i = 0;
        while i < N {
            if let (_, Some(idx)) = self.table.find(self.make_hash(ks[i]), ks[i]) {
                let mut j = 0;
                while j < i {
                    if idxs[j] == idx {
                        crate::rt::panic_str("duplicate keys found", file!(), line!(), column!());
                    }
                    j += 1;
                }
                idxs[i] = idx;
            }
            i += 1;
        }
        let base = self.table.entries.as_mut_ptr();
        crate::array::from_fn(|i| {
            if idxs[i] == usize::MAX {
                None
            } else {
                Some(unsafe { &mut (*base.add(idxs[i])).value })
            }
        })
    }

    /// Inserts a key-value pair into the map, returning the old value if the key was present.
    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        let hash = self.make_hash(&k);
        match self.table.find(hash, &k) {
            (_, Some(idx)) => Some(mem::replace(&mut self.table.entries[idx].value, v)),
            (_, None) => {
                self.table.insert_new(hash, k, v);
                None
            }
        }
    }

    /// Tries to insert a key-value pair into the map, and returns a mutable reference to the value in the entry.
    pub fn try_insert(&mut self, key: K, value: V) -> Result<&mut V, OccupiedError<'_, K, V>> {
        match self.entry(key) {
            Entry::Occupied(entry) => Err(OccupiedError { entry, value }),
            Entry::Vacant(entry) => Ok(entry.insert(value)),
        }
    }

    /// Returns the stored key equal to `key`, inserting `key` with `value` if absent.
    pub(crate) fn get_or_insert_key(&mut self, key: K, value: V) -> &K {
        let hash = self.make_hash(&key);
        let idx = match self.table.find(hash, &key) {
            (_, Some(idx)) => idx,
            (_, None) => self.table.insert_new(hash, key, value),
        };
        &self.table.entries[idx].key
    }

    /// Removes a key from the map, returning the value at the key if the key was previously in the map.
    #[inline]
    pub fn remove<Q: ?Sized>(&mut self, k: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        match self.remove_entry(k) {
            Some((_, v)) => Some(v),
            None => None,
        }
    }

    /// Removes a key from the map, returning the stored key and value if the key was previously in the map.
    pub fn remove_entry<Q: ?Sized>(&mut self, k: &Q) -> Option<(K, V)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        match self.table.find(self.make_hash(k), k) {
            (slot, Some(idx)) => {
                let b = self.table.remove_at(slot, idx);
                Some((b.key, b.value))
            }
            _ => None,
        }
    }
}

/// The error returned by `try_insert` when the key already exists.
pub struct OccupiedError<'a, K: 'a, V: 'a> {
    /// The entry in the map that was already occupied.
    pub entry: OccupiedEntry<'a, K, V>,
    /// The value which was not inserted, because the entry was already occupied.
    pub value: V,
}

impl<'a, K: fmt::Debug, V: fmt::Debug> fmt::Debug for OccupiedError<'a, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let b = &self.entry.map.entries[self.entry.idx];
        f.debug_struct("OccupiedError")
            .field("key", &b.key)
            .field("old_value", &b.value)
            .field("new_value", &self.value)
            .finish()
    }
}

impl<'a, K: fmt::Debug, V: fmt::Debug> fmt::Display for OccupiedError<'a, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let b = &self.entry.map.entries[self.entry.idx];
        write!(
            f,
            "failed to insert {:?}, key {:?} already exists with value {:?}",
            self.value, b.key, b.value,
        )
    }
}

impl<K: Clone, V: Clone, S: Clone> Clone for HashMap<K, V, S> {
    fn clone(&self) -> Self {
        Self { table: self.table.clone(), hash_builder: self.hash_builder.clone() }
    }
}

impl<K, V, S: Default> Default for HashMap<K, V, S> {
    #[inline]
    fn default() -> HashMap<K, V, S> {
        Self { table: RawTable::new(), hash_builder: Default::default() }
    }
}

impl<K: Eq + Hash, V: PartialEq, S: BuildHasher> PartialEq for HashMap<K, V, S> {
    fn eq(&self, other: &HashMap<K, V, S>) -> bool {
        if self.len() != other.len() {
            return false;
        }
        let mut it = self.iter();
        while let Some((key, value)) = it.next() {
            match other.get(key) {
                Some(v) => {
                    if *value != *v {
                        return false;
                    }
                }
                None => return false,
            }
        }
        true
    }
}

impl<K: Eq + Hash, V: Eq, S: BuildHasher> Eq for HashMap<K, V, S> {}

impl<K: fmt::Debug, V: fmt::Debug, S> fmt::Debug for HashMap<K, V, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl<K, Q: ?Sized, V, S> Index<&Q> for HashMap<K, V, S>
where
    K: Eq + Hash + Borrow<Q>,
    Q: Eq + Hash,
    S: BuildHasher,
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

impl<K: Eq + Hash, V, S: BuildHasher + Default> FromIterator<(K, V)> for HashMap<K, V, S> {
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> HashMap<K, V, S> {
        let mut map = Self { table: RawTable::new(), hash_builder: Default::default() };
        map.extend(iter);
        map
    }
}

impl<K: Eq + Hash, V, S: BuildHasher> Extend<(K, V)> for HashMap<K, V, S> {
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

impl<'a, K: Eq + Hash + Copy, V: Copy, S: BuildHasher> Extend<(&'a K, &'a V)> for HashMap<K, V, S> {
    fn extend<T: IntoIterator<Item = (&'a K, &'a V)>>(&mut self, iter: T) {
        let mut it = iter.into_iter();
        while let Some((k, v)) = it.next() {
            self.insert(*k, *v);
        }
    }
}

impl<K: Eq + Hash, V, const N: usize> From<[(K, V); N]> for HashMap<K, V, RandomState> {
    fn from(arr: [(K, V); N]) -> Self {
        let mut map = HashMap::with_capacity(N);
        map.extend(arr);
        map
    }
}

impl<'a, K, V, S> IntoIterator for &'a HashMap<K, V, S> {
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;
    #[inline]
    fn into_iter(self) -> Iter<'a, K, V> {
        self.iter()
    }
}

impl<'a, K, V, S> IntoIterator for &'a mut HashMap<K, V, S> {
    type Item = (&'a K, &'a mut V);
    type IntoIter = IterMut<'a, K, V>;
    #[inline]
    fn into_iter(self) -> IterMut<'a, K, V> {
        self.iter_mut()
    }
}

impl<K, V, S> IntoIterator for HashMap<K, V, S> {
    type Item = (K, V);
    type IntoIter = IntoIter<K, V>;
    #[inline]
    fn into_iter(self) -> IntoIter<K, V> {
        IntoIter { inner: self.table.entries.into_iter() }
    }
}

// ---------------------------------------------------------------------------
// Iterators

/// An iterator over the entries of a `HashMap`.
pub struct Iter<'a, K: 'a, V: 'a> {
    inner: slice::Iter<'a, Bucket<K, V>>,
}

impl<'a, K, V> Clone for Iter<'a, K, V> {
    fn clone(&self) -> Self {
        Iter { inner: self.inner.clone() }
    }
}

impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);
    #[inline]
    fn next(&mut self) -> Option<(&'a K, &'a V)> {
        match self.inner.next() {
            Some(b) => Some((&b.key, &b.value)),
            None => None,
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
    #[inline]
    fn count(self) -> usize {
        self.inner.len()
    }
}

impl<'a, K, V> ExactSizeIterator for Iter<'a, K, V> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<'a, K, V> FusedIterator for Iter<'a, K, V> {}

impl<'a, K: fmt::Debug, V: fmt::Debug> fmt::Debug for Iter<'a, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

/// A mutable iterator over the entries of a `HashMap`.
pub struct IterMut<'a, K: 'a, V: 'a> {
    inner: slice::IterMut<'a, Bucket<K, V>>,
}

impl<'a, K, V> Iterator for IterMut<'a, K, V> {
    type Item = (&'a K, &'a mut V);
    #[inline]
    fn next(&mut self) -> Option<(&'a K, &'a mut V)> {
        match self.inner.next() {
            Some(b) => Some((&b.key, &mut b.value)),
            None => None,
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a, K, V> ExactSizeIterator for IterMut<'a, K, V> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<'a, K, V> FusedIterator for IterMut<'a, K, V> {}

/// An owning iterator over the entries of a `HashMap`.
pub struct IntoIter<K, V> {
    inner: vec::IntoIter<Bucket<K, V>>,
}

impl<K, V> Iterator for IntoIter<K, V> {
    type Item = (K, V);
    #[inline]
    fn next(&mut self) -> Option<(K, V)> {
        match self.inner.next() {
            Some(b) => Some((b.key, b.value)),
            None => None,
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<K, V> ExactSizeIterator for IntoIter<K, V> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<K, V> FusedIterator for IntoIter<K, V> {}

/// A draining iterator over the entries of a `HashMap`.
pub struct Drain<'a, K: 'a, V: 'a> {
    inner: vec::Drain<'a, Bucket<K, V>>,
}

impl<'a, K, V> Iterator for Drain<'a, K, V> {
    type Item = (K, V);
    #[inline]
    fn next(&mut self) -> Option<(K, V)> {
        match self.inner.next() {
            Some(b) => Some((b.key, b.value)),
            None => None,
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a, K, V> ExactSizeIterator for Drain<'a, K, V> {}

/// An iterator over the keys of a `HashMap`.
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

/// An iterator over the values of a `HashMap`.
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

/// A mutable iterator over the values of a `HashMap`.
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

impl<'a, K, V> ExactSizeIterator for ValuesMut<'a, K, V> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

/// An owning iterator over the keys of a `HashMap`.
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

impl<K, V> ExactSizeIterator for IntoKeys<K, V> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

/// An owning iterator over the values of a `HashMap`.
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

impl<K, V> ExactSizeIterator for IntoValues<K, V> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

// ---------------------------------------------------------------------------
// Entry API

/// A view into a single entry in a map, which may either be vacant or occupied.
pub enum Entry<'a, K: 'a, V: 'a> {
    /// An occupied entry.
    Occupied(OccupiedEntry<'a, K, V>),
    /// A vacant entry.
    Vacant(VacantEntry<'a, K, V>),
}

pub use self::Entry::{Occupied, Vacant};

/// A view into an occupied entry in a `HashMap`.
pub struct OccupiedEntry<'a, K: 'a, V: 'a> {
    map: &'a mut RawTable<K, V>,
    idx: usize,
    key: Option<K>,
}

/// A view into a vacant entry in a `HashMap`.
pub struct VacantEntry<'a, K: 'a, V: 'a> {
    map: &'a mut RawTable<K, V>,
    hash: u64,
    key: K,
}

impl<'a, K, V> Entry<'a, K, V> {
    /// Ensures a value is in the entry by inserting the default if empty, and returns a mutable reference to the value in the entry.
    #[inline]
    pub fn or_insert(self, default: V) -> &'a mut V {
        match self {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(default),
        }
    }

    /// Ensures a value is in the entry by inserting the result of the default function if empty.
    #[inline]
    pub fn or_insert_with<F: FnOnce() -> V>(self, default: F) -> &'a mut V {
        match self {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(default()),
        }
    }

    /// Ensures a value is in the entry by inserting, if empty, the result of the default function called with the key.
    #[inline]
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
    #[inline]
    pub fn key(&self) -> &K {
        match *self {
            Entry::Occupied(ref entry) => entry.key(),
            Entry::Vacant(ref entry) => entry.key(),
        }
    }

    /// Provides in-place mutable access to an occupied entry before any potential inserts into the map.
    #[inline]
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
    #[inline]
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

impl<'a, K, V: Default> Entry<'a, K, V> {
    /// Ensures a value is in the entry by inserting the default value if empty.
    #[inline]
    pub fn or_default(self) -> &'a mut V {
        match self {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(Default::default()),
        }
    }
}

impl<'a, K, V> OccupiedEntry<'a, K, V> {
    /// Gets a reference to the key in the entry.
    #[inline]
    pub fn key(&self) -> &K {
        &self.map.entries[self.idx].key
    }

    /// Take the ownership of the key and value from the map.
    pub fn remove_entry(self) -> (K, V) {
        let hash = self.map.entries[self.idx].hash;
        let mask = self.map.indices.len() - 1;
        let mut slot = (hash as usize) & mask;
        while self.map.indices[slot] != self.idx {
            slot = (slot + 1) & mask;
        }
        let b = self.map.remove_at(slot, self.idx);
        (b.key, b.value)
    }

    /// Gets a reference to the value in the entry.
    #[inline]
    pub fn get(&self) -> &V {
        &self.map.entries[self.idx].value
    }

    /// Gets a mutable reference to the value in the entry.
    #[inline]
    pub fn get_mut(&mut self) -> &mut V {
        &mut self.map.entries[self.idx].value
    }

    /// Converts the `OccupiedEntry` into a mutable reference to the value in the entry with a lifetime bound to the map itself.
    #[inline]
    pub fn into_mut(self) -> &'a mut V {
        let idx = self.idx;
        &mut self.map.entries[idx].value
    }

    /// Sets the value of the entry, and returns the entry's old value.
    #[inline]
    pub fn insert(&mut self, value: V) -> V {
        mem::replace(self.get_mut(), value)
    }

    /// Takes the value out of the entry, and returns it.
    #[inline]
    pub fn remove(self) -> V {
        self.remove_entry().1
    }

    /// Replaces the entry, returning the old key and value.
    pub fn replace_entry(self, value: V) -> (K, V) {
        let idx = self.idx;
        let new_key = match self.key {
            Some(k) => k,
            None => crate::rt::panic_str("no key available", file!(), line!(), column!()),
        };
        let b = &mut self.map.entries[idx];
        let old_key = mem::replace(&mut b.key, new_key);
        let old_value = mem::replace(&mut b.value, value);
        (old_key, old_value)
    }
}

impl<'a, K, V> VacantEntry<'a, K, V> {
    /// Gets a reference to the key that would be used when inserting a value through the `VacantEntry`.
    #[inline]
    pub fn key(&self) -> &K {
        &self.key
    }

    /// Take ownership of the key.
    #[inline]
    pub fn into_key(self) -> K {
        self.key
    }

    /// Sets the value of the entry with the `VacantEntry`'s key, and returns a mutable reference to it.
    #[inline]
    pub fn insert(self, value: V) -> &'a mut V {
        let map = self.map;
        let idx = map.insert_new(self.hash, self.key, value);
        &mut map.entries[idx].value
    }

    /// Sets the value of the entry with the `VacantEntry`'s key, and returns an `OccupiedEntry`.
    #[inline]
    pub fn insert_entry(self, value: V) -> OccupiedEntry<'a, K, V> {
        let map = self.map;
        let idx = map.insert_new(self.hash, self.key, value);
        OccupiedEntry { map, idx, key: None }
    }
}

impl<'a, K: fmt::Debug, V: fmt::Debug> fmt::Debug for Entry<'a, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Entry::Vacant(ref v) => f.debug_tuple("Entry").field(v).finish(),
            Entry::Occupied(ref o) => f.debug_tuple("Entry").field(o).finish(),
        }
    }
}

impl<'a, K: fmt::Debug, V: fmt::Debug> fmt::Debug for OccupiedEntry<'a, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let b = &self.map.entries[self.idx];
        f.debug_struct("OccupiedEntry").field("key", &b.key).field("value", &b.value).finish_non_exhaustive()
    }
}

impl<'a, K: fmt::Debug, V> fmt::Debug for VacantEntry<'a, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("VacantEntry").field(&self.key).finish()
    }
}
