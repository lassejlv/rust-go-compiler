//! A hash set implemented as a `HashMap` where the value is `()`.

use super::hash_map::{self, HashMap};
use crate::borrow::Borrow;
use crate::fmt;
use crate::hash::{BuildHasher, Hash, RandomState};
use crate::iter::{Chain, ExactSizeIterator, Extend, FromIterator, FusedIterator, IntoIterator, Iterator};
use crate::ops::{BitAnd, BitOr, BitXor, Sub};

/// A hash set.
pub struct HashSet<T, S = RandomState> {
    map: HashMap<T, (), S>,
}

impl<T> HashSet<T, RandomState> {
    /// Creates an empty `HashSet`.
    #[inline]
    pub fn new() -> HashSet<T, RandomState> {
        Self { map: HashMap::new() }
    }

    /// Creates an empty `HashSet` with at least the specified capacity.
    #[inline]
    pub fn with_capacity(capacity: usize) -> HashSet<T, RandomState> {
        Self { map: HashMap::with_capacity(capacity) }
    }
}

impl<T, S> HashSet<T, S> {
    /// Creates a new empty hash set which will use the given hasher to hash keys.
    #[inline]
    pub const fn with_hasher(hasher: S) -> HashSet<T, S> {
        Self { map: HashMap::<T, (), S>::with_hasher(hasher) }
    }

    /// Creates an empty `HashSet` with at least the specified capacity, using `hasher` to hash the keys.
    #[inline]
    pub fn with_capacity_and_hasher(capacity: usize, hasher: S) -> HashSet<T, S> {
        Self { map: HashMap::<T, (), S>::with_capacity_and_hasher(capacity, hasher) }
    }

    /// Returns a reference to the set's `BuildHasher`.
    #[inline]
    pub fn hasher(&self) -> &S {
        self.map.hasher()
    }

    /// Returns the number of elements the set can hold without reallocating.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.map.capacity()
    }

    /// An iterator visiting all elements in arbitrary order.
    #[inline]
    pub fn iter(&self) -> Iter<'_, T> {
        Iter { inner: self.map.keys() }
    }

    /// Returns the number of elements in the set.
    #[inline]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Returns `true` if the set contains no elements.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Clears the set, returning all elements as an iterator.
    #[inline]
    pub fn drain(&mut self) -> Drain<'_, T> {
        Drain { inner: self.map.drain() }
    }

    /// Retains only the elements specified by the predicate.
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&T) -> bool,
    {
        let mut f = f;
        self.map.retain(|k, _| f(k));
    }

    /// Creates an iterator which uses a closure to determine if a value should be removed
    /// (eagerly in rustgo).
    pub fn extract_if<F>(&mut self, pred: F) -> crate::iter::Map<crate::vec::IntoIter<(T, ())>, fn((T, ())) -> T>
    where
        F: FnMut(&T) -> bool,
    {
        let mut pred = pred;
        let f: fn((T, ())) -> T = |(k, _)| k;
        self.map.extract_if(|k, _| pred(k)).map(f)
    }

    /// Clears the set, removing all values.
    #[inline]
    pub fn clear(&mut self) {
        self.map.clear()
    }
}

impl<T: Eq + Hash, S: BuildHasher> HashSet<T, S> {
    /// Reserves capacity for at least `additional` more elements.
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.map.reserve(additional)
    }

    /// Shrinks the capacity of the set as much as possible.
    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.map.shrink_to_fit()
    }

    /// Visits the values representing the difference, i.e., the values that are in `self` but not in `other`.
    #[inline]
    pub fn difference<'a>(&'a self, other: &'a HashSet<T, S>) -> Difference<'a, T, S> {
        Difference { iter: self.iter(), other }
    }

    /// Visits the values representing the symmetric difference.
    #[inline]
    pub fn symmetric_difference<'a>(&'a self, other: &'a HashSet<T, S>) -> SymmetricDifference<'a, T, S> {
        SymmetricDifference { iter: self.difference(other).chain(other.difference(self)) }
    }

    /// Visits the values representing the intersection.
    #[inline]
    pub fn intersection<'a>(&'a self, other: &'a HashSet<T, S>) -> Intersection<'a, T, S> {
        if self.len() <= other.len() {
            Intersection { iter: self.iter(), other }
        } else {
            Intersection { iter: other.iter(), other: self }
        }
    }

    /// Visits the values representing the union.
    #[inline]
    pub fn union<'a>(&'a self, other: &'a HashSet<T, S>) -> Union<'a, T, S> {
        if self.len() >= other.len() {
            Union { iter: self.iter().chain(other.difference(self)) }
        } else {
            Union { iter: other.iter().chain(self.difference(other)) }
        }
    }

    /// Returns `true` if the set contains a value.
    #[inline]
    pub fn contains<Q: ?Sized>(&self, value: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.map.contains_key(value)
    }

    /// Returns a reference to the value in the set, if any, that is equal to the given value.
    #[inline]
    pub fn get<Q: ?Sized>(&self, value: &Q) -> Option<&T>
    where
        T: Borrow<Q>,
        Q: Hash + Eq,
    {
        match self.map.get_key_value(value) {
            Some((k, _)) => Some(k),
            None => None,
        }
    }

    /// Inserts the given `value` into the set if it is not present, then returns a reference to the value in the set.
    #[inline]
    pub fn get_or_insert(&mut self, value: T) -> &T {
        self.map.get_or_insert_key(value, ())
    }

    /// Returns `true` if `self` has no elements in common with `other`.
    pub fn is_disjoint(&self, other: &HashSet<T, S>) -> bool {
        let mut it = self.iter();
        while let Some(v) = it.next() {
            if other.contains(v) {
                return false;
            }
        }
        true
    }

    /// Returns `true` if the set is a subset of another.
    pub fn is_subset(&self, other: &HashSet<T, S>) -> bool {
        if self.len() > other.len() {
            return false;
        }
        let mut it = self.iter();
        while let Some(v) = it.next() {
            if !other.contains(v) {
                return false;
            }
        }
        true
    }

    /// Returns `true` if the set is a superset of another.
    #[inline]
    pub fn is_superset(&self, other: &HashSet<T, S>) -> bool {
        other.is_subset(self)
    }

    /// Adds a value to the set. Returns whether the value was newly inserted.
    #[inline]
    pub fn insert(&mut self, value: T) -> bool {
        match self.map.entry(value) {
            hash_map::Entry::Occupied(_) => false,
            hash_map::Entry::Vacant(e) => {
                e.insert(());
                true
            }
        }
    }

    /// Adds a value to the set, replacing the existing value, if any, that is equal to the given one.
    pub fn replace(&mut self, value: T) -> Option<T> {
        match self.map.entry(value) {
            hash_map::Entry::Occupied(occupied) => Some(occupied.replace_entry(()).0),
            hash_map::Entry::Vacant(vacant) => {
                vacant.insert(());
                None
            }
        }
    }

    /// Removes a value from the set. Returns whether the value was present in the set.
    #[inline]
    pub fn remove<Q: ?Sized>(&mut self, value: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.map.remove(value).is_some()
    }

    /// Removes and returns the value in the set, if any, that is equal to the given one.
    #[inline]
    pub fn take<Q: ?Sized>(&mut self, value: &Q) -> Option<T>
    where
        T: Borrow<Q>,
        Q: Hash + Eq,
    {
        match self.map.remove_entry(value) {
            Some((k, _)) => Some(k),
            None => None,
        }
    }
}

impl<T: Clone, S: Clone> Clone for HashSet<T, S> {
    fn clone(&self) -> Self {
        Self { map: self.map.clone() }
    }
}

impl<T: Eq + Hash, S: BuildHasher> PartialEq for HashSet<T, S> {
    fn eq(&self, other: &HashSet<T, S>) -> bool {
        if self.len() != other.len() {
            return false;
        }
        let mut it = self.iter();
        while let Some(key) = it.next() {
            if !other.contains(key) {
                return false;
            }
        }
        true
    }
}

impl<T: Eq + Hash, S: BuildHasher> Eq for HashSet<T, S> {}

impl<T: fmt::Debug, S> fmt::Debug for HashSet<T, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

impl<T: Eq + Hash, S: BuildHasher + Default> FromIterator<T> for HashSet<T, S> {
    #[inline]
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> HashSet<T, S> {
        let mut set = Self { map: Default::default() };
        set.extend(iter);
        set
    }
}

impl<T: Eq + Hash, const N: usize> From<[T; N]> for HashSet<T, RandomState> {
    fn from(arr: [T; N]) -> Self {
        let mut set = HashSet::with_capacity(N);
        set.extend(arr);
        set
    }
}

impl<T: Eq + Hash, S: BuildHasher> Extend<T> for HashSet<T, S> {
    #[inline]
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        let mut it = iter.into_iter();
        self.map.reserve_for_extend(it.size_hint().0);
        while let Some(k) = it.next() {
            self.insert(k);
        }
    }
    #[inline]
    fn extend_one(&mut self, item: T) {
        self.insert(item);
    }
}

impl<'a, T: 'a + Eq + Hash + Copy, S: BuildHasher> Extend<&'a T> for HashSet<T, S> {
    #[inline]
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, iter: I) {
        let mut it = iter.into_iter();
        self.map.reserve_for_extend(it.size_hint().0);
        while let Some(k) = it.next() {
            self.insert(*k);
        }
    }
}

impl<T, S: Default> Default for HashSet<T, S> {
    #[inline]
    fn default() -> HashSet<T, S> {
        Self { map: Default::default() }
    }
}

impl<T: Eq + Hash + Clone, S: BuildHasher + Default> BitOr<&HashSet<T, S>> for &HashSet<T, S> {
    type Output = HashSet<T, S>;
    fn bitor(self, rhs: &HashSet<T, S>) -> HashSet<T, S> {
        self.union(rhs).cloned().collect()
    }
}

impl<T: Eq + Hash + Clone, S: BuildHasher + Default> BitAnd<&HashSet<T, S>> for &HashSet<T, S> {
    type Output = HashSet<T, S>;
    fn bitand(self, rhs: &HashSet<T, S>) -> HashSet<T, S> {
        self.intersection(rhs).cloned().collect()
    }
}

impl<T: Eq + Hash + Clone, S: BuildHasher + Default> BitXor<&HashSet<T, S>> for &HashSet<T, S> {
    type Output = HashSet<T, S>;
    fn bitxor(self, rhs: &HashSet<T, S>) -> HashSet<T, S> {
        self.symmetric_difference(rhs).cloned().collect()
    }
}

impl<T: Eq + Hash + Clone, S: BuildHasher + Default> Sub<&HashSet<T, S>> for &HashSet<T, S> {
    type Output = HashSet<T, S>;
    fn sub(self, rhs: &HashSet<T, S>) -> HashSet<T, S> {
        self.difference(rhs).cloned().collect()
    }
}

impl<'a, T, S> IntoIterator for &'a HashSet<T, S> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;
    #[inline]
    fn into_iter(self) -> Iter<'a, T> {
        self.iter()
    }
}

impl<T, S> IntoIterator for HashSet<T, S> {
    type Item = T;
    type IntoIter = IntoIter<T>;
    #[inline]
    fn into_iter(self) -> IntoIter<T> {
        IntoIter { inner: self.map.into_keys() }
    }
}

/// An iterator over the items of a `HashSet`.
pub struct Iter<'a, K: 'a> {
    inner: hash_map::Keys<'a, K, ()>,
}

impl<'a, K> Clone for Iter<'a, K> {
    fn clone(&self) -> Self {
        Iter { inner: self.inner.clone() }
    }
}

impl<'a, K> Iterator for Iter<'a, K> {
    type Item = &'a K;
    #[inline]
    fn next(&mut self) -> Option<&'a K> {
        self.inner.next()
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a, K> ExactSizeIterator for Iter<'a, K> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<'a, K> FusedIterator for Iter<'a, K> {}

impl<'a, K: fmt::Debug> fmt::Debug for Iter<'a, K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

/// An owning iterator over the items of a `HashSet`.
pub struct IntoIter<K> {
    inner: hash_map::IntoKeys<K, ()>,
}

impl<K: fmt::Debug> fmt::Debug for IntoIter<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.inner, f)
    }
}

impl<K> Iterator for IntoIter<K> {
    type Item = K;
    #[inline]
    fn next(&mut self) -> Option<K> {
        self.inner.next()
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<K> ExactSizeIterator for IntoIter<K> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

/// A draining iterator over the items of a `HashSet`.
pub struct Drain<'a, K: 'a> {
    inner: hash_map::Drain<'a, K, ()>,
}

impl<'a, K: fmt::Debug> fmt::Debug for Drain<'a, K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.inner.remaining_keys()).finish()
    }
}

impl<'a, K> Iterator for Drain<'a, K> {
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

/// A lazy iterator producing elements in the intersection of `HashSet`s.
pub struct Intersection<'a, T: 'a, S: 'a = RandomState> {
    iter: Iter<'a, T>,
    other: &'a HashSet<T, S>,
}

impl<'a, T, S> Clone for Intersection<'a, T, S> {
    fn clone(&self) -> Self {
        Intersection { iter: self.iter.clone(), other: self.other }
    }
}

impl<'a, T: Eq + Hash, S: BuildHasher> Iterator for Intersection<'a, T, S> {
    type Item = &'a T;
    #[inline]
    fn next(&mut self) -> Option<&'a T> {
        loop {
            let elt = match self.iter.next() {
                Some(e) => e,
                None => return None,
            };
            if self.other.contains(elt) {
                return Some(elt);
            }
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (_, upper) = self.iter.size_hint();
        (0, upper)
    }
}

impl<'a, T: fmt::Debug + Eq + Hash, S: BuildHasher> fmt::Debug for Intersection<'a, T, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

/// A lazy iterator producing elements in the difference of `HashSet`s.
pub struct Difference<'a, T: 'a, S: 'a = RandomState> {
    iter: Iter<'a, T>,
    other: &'a HashSet<T, S>,
}

impl<'a, T, S> Clone for Difference<'a, T, S> {
    fn clone(&self) -> Self {
        Difference { iter: self.iter.clone(), other: self.other }
    }
}

impl<'a, T: Eq + Hash, S: BuildHasher> Iterator for Difference<'a, T, S> {
    type Item = &'a T;
    #[inline]
    fn next(&mut self) -> Option<&'a T> {
        loop {
            let elt = match self.iter.next() {
                Some(e) => e,
                None => return None,
            };
            if !self.other.contains(elt) {
                return Some(elt);
            }
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (_, upper) = self.iter.size_hint();
        (0, upper)
    }
}

impl<'a, T: fmt::Debug + Eq + Hash, S: BuildHasher> fmt::Debug for Difference<'a, T, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

/// A lazy iterator producing elements in the symmetric difference of `HashSet`s.
pub struct SymmetricDifference<'a, T: 'a, S: 'a = RandomState> {
    iter: Chain<Difference<'a, T, S>, Difference<'a, T, S>>,
}

impl<'a, T, S> Clone for SymmetricDifference<'a, T, S> {
    fn clone(&self) -> Self {
        SymmetricDifference { iter: self.iter.clone() }
    }
}

impl<'a, T: Eq + Hash, S: BuildHasher> Iterator for SymmetricDifference<'a, T, S> {
    type Item = &'a T;
    #[inline]
    fn next(&mut self) -> Option<&'a T> {
        self.iter.next()
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'a, T: fmt::Debug + Eq + Hash, S: BuildHasher> fmt::Debug for SymmetricDifference<'a, T, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

/// A lazy iterator producing elements in the union of `HashSet`s.
pub struct Union<'a, T: 'a, S: 'a = RandomState> {
    iter: Chain<Iter<'a, T>, Difference<'a, T, S>>,
}

impl<'a, T, S> Clone for Union<'a, T, S> {
    fn clone(&self) -> Self {
        Union { iter: self.iter.clone() }
    }
}

impl<'a, T: Eq + Hash, S: BuildHasher> Iterator for Union<'a, T, S> {
    type Item = &'a T;
    #[inline]
    fn next(&mut self) -> Option<&'a T> {
        self.iter.next()
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'a, T: fmt::Debug + Eq + Hash, S: BuildHasher> fmt::Debug for Union<'a, T, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

impl<'a, K> Default for Iter<'a, K> {
    fn default() -> Self {
        Iter { inner: Default::default() }
    }
}

impl<K> Default for IntoIter<K> {
    fn default() -> Self {
        IntoIter { inner: Default::default() }
    }
}
