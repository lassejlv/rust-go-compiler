//! An ordered set (a `BTreeMap` with `()` values).

use super::btree_map::{self, BTreeMap};
use crate::borrow::Borrow;
use crate::cmp::Ordering;
use crate::fmt;
use crate::hash::{Hash, Hasher};
use crate::iter::{DoubleEndedIterator, ExactSizeIterator, Extend, FromIterator, FusedIterator, IntoIterator, Iterator};
use crate::ops::{BitAnd, BitOr, BitXor, RangeBounds, Sub};

/// An ordered set.
pub struct BTreeSet<T> {
    map: BTreeMap<T, ()>,
}

impl<T> BTreeSet<T> {
    /// Makes a new, empty `BTreeSet`.
    #[inline]
    pub const fn new() -> BTreeSet<T> {
        BTreeSet { map: BTreeMap::new() }
    }

    /// Gets an iterator that visits the elements in the `BTreeSet` in ascending order.
    #[inline]
    pub fn iter(&self) -> Iter<'_, T> {
        Iter { inner: self.map.keys() }
    }

    /// Returns the number of elements in the set.
    #[inline]
    pub const fn len(&self) -> usize {
        self.map.len()
    }

    /// Returns `true` if the set contains no elements.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Clears the set, removing all elements.
    #[inline]
    pub fn clear(&mut self) {
        self.map.clear()
    }

    /// Returns a reference to the first element in the set, if any (the minimum).
    #[inline]
    pub fn first(&self) -> Option<&T> {
        match self.map.first_key_value() {
            Some((k, _)) => Some(k),
            None => None,
        }
    }

    /// Returns a reference to the last element in the set, if any (the maximum).
    #[inline]
    pub fn last(&self) -> Option<&T> {
        match self.map.last_key_value() {
            Some((k, _)) => Some(k),
            None => None,
        }
    }

    /// Removes the first element from the set and returns it, if any.
    #[inline]
    pub fn pop_first(&mut self) -> Option<T> {
        match self.map.pop_first() {
            Some((k, _)) => Some(k),
            None => None,
        }
    }

    /// Removes the last element from the set and returns it, if any.
    #[inline]
    pub fn pop_last(&mut self) -> Option<T> {
        match self.map.pop_last() {
            Some((k, _)) => Some(k),
            None => None,
        }
    }

    /// Retains only the elements specified by the predicate.
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&T) -> bool,
    {
        let mut f = f;
        self.map.retain(|k, _| f(k));
    }

    /// Creates an iterator that visits elements in `range` in ascending order and uses a
    /// closure to determine if an element should be removed (eagerly in rustgo).
    pub fn extract_if<F, R>(&mut self, range: R, pred: F) -> crate::iter::Map<crate::vec::IntoIter<(T, ())>, fn((T, ())) -> T>
    where
        T: Ord,
        R: RangeBounds<T>,
        F: FnMut(&T) -> bool,
    {
        let mut pred = pred;
        let f: fn((T, ())) -> T = |(k, _)| k;
        self.map.extract_if(range, |k, _| pred(k)).map(f)
    }
}

impl<T: Ord> BTreeSet<T> {
    /// Constructs a double-ended iterator over a sub-range of elements in the set.
    pub fn range<K: ?Sized, R>(&self, range: R) -> Range<'_, T>
    where
        K: Ord,
        T: Borrow<K>,
        R: RangeBounds<K>,
    {
        Range { inner: self.map.range(range) }
    }

    /// Visits the elements representing the difference, in ascending order.
    pub fn difference<'a>(&'a self, other: &'a BTreeSet<T>) -> Difference<'a, T> {
        Difference { iter: self.iter(), other }
    }

    /// Visits the elements representing the symmetric difference, in ascending order.
    pub fn symmetric_difference<'a>(&'a self, other: &'a BTreeSet<T>) -> SymmetricDifference<'a, T> {
        SymmetricDifference { a: self.iter(), b: other.iter(), peeked_a: None, peeked_b: None }
    }

    /// Visits the elements representing the intersection, in ascending order.
    pub fn intersection<'a>(&'a self, other: &'a BTreeSet<T>) -> Intersection<'a, T> {
        Intersection { iter: self.iter(), other }
    }

    /// Visits the elements representing the union, in ascending order.
    pub fn union<'a>(&'a self, other: &'a BTreeSet<T>) -> Union<'a, T> {
        Union { a: self.iter(), b: other.iter(), peeked_a: None, peeked_b: None }
    }

    /// Returns `true` if the set contains an element equal to the value.
    #[inline]
    pub fn contains<Q: ?Sized>(&self, value: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: Ord,
    {
        self.map.contains_key(value)
    }

    /// Returns a reference to the element in the set, if any, that is equal to the value.
    pub fn get<Q: ?Sized>(&self, value: &Q) -> Option<&T>
    where
        T: Borrow<Q>,
        Q: Ord,
    {
        match self.map.get_key_value(value) {
            Some((k, _)) => Some(k),
            None => None,
        }
    }

    /// Returns `true` if `self` has no elements in common with `other`.
    pub fn is_disjoint(&self, other: &BTreeSet<T>) -> bool {
        self.intersection(other).next().is_none()
    }

    /// Returns `true` if the set is a subset of another.
    pub fn is_subset(&self, other: &BTreeSet<T>) -> bool {
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
    pub fn is_superset(&self, other: &BTreeSet<T>) -> bool {
        other.is_subset(self)
    }

    /// Adds a value to the set. Returns whether the value was newly inserted.
    pub fn insert(&mut self, value: T) -> bool {
        match self.map.entry(value) {
            btree_map::Entry::Occupied(_) => false,
            btree_map::Entry::Vacant(e) => {
                e.insert(());
                true
            }
        }
    }

    /// Adds a value to the set, replacing the existing element, if any, that is equal to the value.
    pub fn replace(&mut self, value: T) -> Option<T> {
        let old = self.map.remove_entry(&value);
        self.map.insert(value, ());
        match old {
            Some((k, _)) => Some(k),
            None => None,
        }
    }

    /// If the set contains an element equal to the value, removes it from the set and drops it.
    pub fn remove<Q: ?Sized>(&mut self, value: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: Ord,
    {
        self.map.remove(value).is_some()
    }

    /// Removes and returns the element in the set, if any, that is equal to the value.
    pub fn take<Q: ?Sized>(&mut self, value: &Q) -> Option<T>
    where
        T: Borrow<Q>,
        Q: Ord,
    {
        match self.map.remove_entry(value) {
            Some((k, _)) => Some(k),
            None => None,
        }
    }

    /// Moves all elements from `other` into `self`, leaving `other` empty.
    pub fn append(&mut self, other: &mut BTreeSet<T>) {
        self.map.append(&mut other.map);
    }

    /// Splits the collection into two at the value. Returns a new collection with all elements greater than or equal to the value.
    pub fn split_off<Q: ?Sized + Ord>(&mut self, value: &Q) -> BTreeSet<T>
    where
        T: Borrow<Q>,
    {
        BTreeSet { map: self.map.split_off(value) }
    }
}

impl<T: Clone> Clone for BTreeSet<T> {
    fn clone(&self) -> Self {
        BTreeSet { map: self.map.clone() }
    }
}

impl<T> Default for BTreeSet<T> {
    fn default() -> BTreeSet<T> {
        BTreeSet::new()
    }
}

impl<T: PartialEq> PartialEq for BTreeSet<T> {
    fn eq(&self, other: &BTreeSet<T>) -> bool {
        self.map == other.map
    }
}

impl<T: Eq> Eq for BTreeSet<T> {}

impl<T: PartialOrd> PartialOrd for BTreeSet<T> {
    fn partial_cmp(&self, other: &BTreeSet<T>) -> Option<Ordering> {
        self.iter().partial_cmp(other.iter())
    }
}

impl<T: Ord> Ord for BTreeSet<T> {
    fn cmp(&self, other: &BTreeSet<T>) -> Ordering {
        self.iter().cmp(other.iter())
    }
}

impl<T: Hash> Hash for BTreeSet<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_length_prefix(self.len());
        let mut it = self.iter();
        while let Some(elt) = it.next() {
            elt.hash(state);
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for BTreeSet<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

impl<T: Ord> FromIterator<T> for BTreeSet<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> BTreeSet<T> {
        let mut set = BTreeSet::new();
        set.extend(iter);
        set
    }
}

impl<T: Ord, const N: usize> From<[T; N]> for BTreeSet<T> {
    fn from(arr: [T; N]) -> Self {
        let mut set = BTreeSet::new();
        set.extend(arr);
        set
    }
}

impl<T: Ord> Extend<T> for BTreeSet<T> {
    #[inline]
    fn extend<Iter: IntoIterator<Item = T>>(&mut self, iter: Iter) {
        let mut it = iter.into_iter();
        while let Some(elem) = it.next() {
            self.insert(elem);
        }
    }
    #[inline]
    fn extend_one(&mut self, elem: T) {
        self.insert(elem);
    }
}

impl<'a, T: 'a + Ord + Copy> Extend<&'a T> for BTreeSet<T> {
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, iter: I) {
        let mut it = iter.into_iter();
        while let Some(elem) = it.next() {
            self.insert(*elem);
        }
    }
}

impl<T: Ord + Clone> Sub<&BTreeSet<T>> for &BTreeSet<T> {
    type Output = BTreeSet<T>;
    fn sub(self, rhs: &BTreeSet<T>) -> BTreeSet<T> {
        self.difference(rhs).cloned().collect()
    }
}

impl<T: Ord + Clone> BitXor<&BTreeSet<T>> for &BTreeSet<T> {
    type Output = BTreeSet<T>;
    fn bitxor(self, rhs: &BTreeSet<T>) -> BTreeSet<T> {
        self.symmetric_difference(rhs).cloned().collect()
    }
}

impl<T: Ord + Clone> BitAnd<&BTreeSet<T>> for &BTreeSet<T> {
    type Output = BTreeSet<T>;
    fn bitand(self, rhs: &BTreeSet<T>) -> BTreeSet<T> {
        self.intersection(rhs).cloned().collect()
    }
}

impl<T: Ord + Clone> BitOr<&BTreeSet<T>> for &BTreeSet<T> {
    type Output = BTreeSet<T>;
    fn bitor(self, rhs: &BTreeSet<T>) -> BTreeSet<T> {
        self.union(rhs).cloned().collect()
    }
}

impl<'a, T> IntoIterator for &'a BTreeSet<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;
    fn into_iter(self) -> Iter<'a, T> {
        self.iter()
    }
}

impl<T> IntoIterator for BTreeSet<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;
    fn into_iter(self) -> IntoIter<T> {
        IntoIter { inner: self.map.into_keys() }
    }
}

/// An iterator over the items of a `BTreeSet`.
pub struct Iter<'a, T: 'a> {
    inner: btree_map::Keys<'a, T, ()>,
}

impl<'a, T> Clone for Iter<'a, T> {
    fn clone(&self) -> Self {
        Iter { inner: self.inner.clone() }
    }
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;
    #[inline]
    fn next(&mut self) -> Option<&'a T> {
        self.inner.next()
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
    #[inline]
    fn last(self) -> Option<&'a T> {
        let mut s = self;
        s.next_back()
    }
}

impl<'a, T> DoubleEndedIterator for Iter<'a, T> {
    #[inline]
    fn next_back(&mut self) -> Option<&'a T> {
        self.inner.next_back()
    }
}

impl<'a, T> ExactSizeIterator for Iter<'a, T> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<'a, T> FusedIterator for Iter<'a, T> {}

impl<'a, T: fmt::Debug> fmt::Debug for Iter<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Iter").field(&self.clone().collect::<Vec<_>>()).finish()
    }
}

/// An owning iterator over the items of a `BTreeSet` in ascending order.
pub struct IntoIter<T> {
    inner: btree_map::IntoKeys<T, ()>,
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;
    #[inline]
    fn next(&mut self) -> Option<T> {
        self.inner.next()
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<T> DoubleEndedIterator for IntoIter<T> {
    #[inline]
    fn next_back(&mut self) -> Option<T> {
        self.inner.next_back()
    }
}

/// Std's BTreeSet is a `BTreeMap<T, SetValZST>`; its derived iterator Debug
/// shows `(key, SetValZST)` pairs.
#[derive(Debug)]
struct SetValZST;

struct SetPairs<I>(I);

impl<'b, T: fmt::Debug + 'b, I: Iterator<Item = &'b T> + Clone> fmt::Debug for SetPairs<I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.0.clone().map(|k| (k, SetValZST))).finish()
    }
}

impl<T: fmt::Debug> fmt::Debug for IntoIter<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let keys: Vec<&T> = self.inner.remaining_keys().collect();
        f.debug_struct("IntoIter").field("iter", &SetPairs(keys.into_iter())).finish()
    }
}

/// An iterator over a sub-range of items in a `BTreeSet`.
pub struct Range<'a, T: 'a> {
    inner: btree_map::Range<'a, T, ()>,
}

impl<'a, T: fmt::Debug> fmt::Debug for Range<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Range").field("iter", &SetPairs(self.clone())).finish()
    }
}

impl<'a, T> Clone for Range<'a, T> {
    fn clone(&self) -> Self {
        Range { inner: self.inner.clone() }
    }
}

impl<'a, T> Iterator for Range<'a, T> {
    type Item = &'a T;
    #[inline]
    fn next(&mut self) -> Option<&'a T> {
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

impl<'a, T> DoubleEndedIterator for Range<'a, T> {
    #[inline]
    fn next_back(&mut self) -> Option<&'a T> {
        match self.inner.next_back() {
            Some((k, _)) => Some(k),
            None => None,
        }
    }
}

/// A lazy iterator producing elements in the difference of `BTreeSet`s.
pub struct Difference<'a, T: 'a> {
    iter: Iter<'a, T>,
    other: &'a BTreeSet<T>,
}

impl<'a, T> Clone for Difference<'a, T> {
    fn clone(&self) -> Self {
        Difference { iter: self.iter.clone(), other: self.other }
    }
}

impl<'a, T: Ord> Iterator for Difference<'a, T> {
    type Item = &'a T;
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
}

impl<'a, T: fmt::Debug + Ord> fmt::Debug for Difference<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

/// A lazy iterator producing elements in the intersection of `BTreeSet`s.
pub struct Intersection<'a, T: 'a> {
    iter: Iter<'a, T>,
    other: &'a BTreeSet<T>,
}

impl<'a, T> Clone for Intersection<'a, T> {
    fn clone(&self) -> Self {
        Intersection { iter: self.iter.clone(), other: self.other }
    }
}

impl<'a, T: Ord> Iterator for Intersection<'a, T> {
    type Item = &'a T;
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
}

impl<'a, T: fmt::Debug + Ord> fmt::Debug for Intersection<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

/// A lazy iterator producing elements in the union of `BTreeSet`s.
pub struct Union<'a, T: 'a> {
    a: Iter<'a, T>,
    b: Iter<'a, T>,
    peeked_a: Option<&'a T>,
    peeked_b: Option<&'a T>,
}

impl<'a, T> Clone for Union<'a, T> {
    fn clone(&self) -> Self {
        Union { a: self.a.clone(), b: self.b.clone(), peeked_a: self.peeked_a, peeked_b: self.peeked_b }
    }
}

impl<'a, T: Ord> Iterator for Union<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<&'a T> {
        if self.peeked_a.is_none() {
            self.peeked_a = self.a.next();
        }
        if self.peeked_b.is_none() {
            self.peeked_b = self.b.next();
        }
        match (self.peeked_a, self.peeked_b) {
            (None, None) => None,
            (Some(x), None) => {
                self.peeked_a = None;
                Some(x)
            }
            (None, Some(y)) => {
                self.peeked_b = None;
                Some(y)
            }
            (Some(x), Some(y)) => match x.cmp(y) {
                Ordering::Less => {
                    self.peeked_a = None;
                    Some(x)
                }
                Ordering::Greater => {
                    self.peeked_b = None;
                    Some(y)
                }
                Ordering::Equal => {
                    self.peeked_a = None;
                    self.peeked_b = None;
                    Some(x)
                }
            },
        }
    }
}

impl<'a, T: fmt::Debug + Ord> fmt::Debug for Union<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

/// A lazy iterator producing elements in the symmetric difference of `BTreeSet`s.
pub struct SymmetricDifference<'a, T: 'a> {
    a: Iter<'a, T>,
    b: Iter<'a, T>,
    peeked_a: Option<&'a T>,
    peeked_b: Option<&'a T>,
}

impl<'a, T> Clone for SymmetricDifference<'a, T> {
    fn clone(&self) -> Self {
        SymmetricDifference { a: self.a.clone(), b: self.b.clone(), peeked_a: self.peeked_a, peeked_b: self.peeked_b }
    }
}

impl<'a, T: Ord> Iterator for SymmetricDifference<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<&'a T> {
        loop {
            if self.peeked_a.is_none() {
                self.peeked_a = self.a.next();
            }
            if self.peeked_b.is_none() {
                self.peeked_b = self.b.next();
            }
            match (self.peeked_a, self.peeked_b) {
                (None, None) => return None,
                (Some(x), None) => {
                    self.peeked_a = None;
                    return Some(x);
                }
                (None, Some(y)) => {
                    self.peeked_b = None;
                    return Some(y);
                }
                (Some(x), Some(y)) => match x.cmp(y) {
                    Ordering::Less => {
                        self.peeked_a = None;
                        return Some(x);
                    }
                    Ordering::Greater => {
                        self.peeked_b = None;
                        return Some(y);
                    }
                    Ordering::Equal => {
                        self.peeked_a = None;
                        self.peeked_b = None;
                    }
                },
            }
        }
    }
}

impl<'a, T: fmt::Debug + Ord> fmt::Debug for SymmetricDifference<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}
