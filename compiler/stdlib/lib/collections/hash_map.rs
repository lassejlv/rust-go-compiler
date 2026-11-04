//! A hash map: open addressing over one allocation of buckets with
//! SwissTable control bytes (one per bucket: EMPTY, DELETED or the top 7
//! bits of the hash), probed a group of 8 buckets at a time with SWAR bit
//! tricks, like hashbrown's portable implementation (which the real library
//! uses). Capacity, growth (load factor 7/8) and the iteration order
//! (bucket order) follow hashbrown; the order is unspecified, as in the
//! real library.

use crate::borrow::Borrow;
use crate::fmt;
use crate::hash::{BuildHasher, Hash};
use crate::intrinsics;

use crate::iter::{ExactSizeIterator, Extend, FromIterator, FusedIterator, IntoIterator, Iterator};
use crate::marker::PhantomData;
use crate::mem;
use crate::ops::Index;
use crate::vec;

pub use crate::hash::{DefaultHasher, RandomState};

// ---------------------------------------------------------------------------
// Control bytes and groups

const EMPTY: u8 = 0xFF;
const DELETED: u8 = 0x80;
const GROUP: usize = 8;
const LO: u64 = 0x0101010101010101;
const HI: u64 = 0x8080808080808080;

/// The control byte of a full bucket: the top 7 bits of the hash.
#[inline(always)]
fn h2(hash: u64) -> u8 {
    (hash >> 57) as u8
}

#[inline(always)]
fn is_full(c: u8) -> bool {
    c & 0x80 == 0
}

/// Loads the 8 control bytes at `p` (little endian: byte i is bits 8i..8i+8).
#[inline(always)]
unsafe fn load_group(p: *const u8) -> u64 {
    let mut g: u64 = 0;
    intrinsics::copy_nonoverlapping(p, &mut g as *mut u64 as *mut u8, 8);
    u64::from_le(g)
}

/// The bytes of `g` equal to `b` (high bit of each; rare false positives
/// are filtered by the key comparison).
#[inline(always)]
fn match_byte(g: u64, b: u8) -> u64 {
    let cmp = g ^ LO.wrapping_mul(b as u64);
    cmp.wrapping_sub(LO) & !cmp & HI
}

#[inline(always)]
fn match_empty(g: u64) -> u64 {
    g & (g << 1) & HI
}

#[inline(always)]
fn match_empty_or_deleted(g: u64) -> u64 {
    g & HI
}

#[inline(always)]
fn match_full(g: u64) -> u64 {
    !g & HI
}

/// The index (in the group) of the lowest byte set in a match.
#[inline(always)]
fn lowest(bits: u64) -> usize {
    (bits.trailing_zeros() / 8) as usize
}

/// The number of buckets for a capacity (hashbrown's capacity_to_buckets,
/// with x86-64's 16-byte groups: small tables of small elements get a larger
/// minimum capacity); `elem_size` is the size of an element.
fn capacity_to_buckets(cap: usize, elem_size: usize) -> Option<usize> {
    if cap < 15 {
        let min_cap = if elem_size <= 1 {
            14
        } else if elem_size <= 3 {
            7
        } else {
            3
        };
        let cap = if cap > min_cap { cap } else { min_cap };
        return Some(if cap < 4 {
            4
        } else if cap < 8 {
            8
        } else {
            16
        });
    }
    let adjusted = cap.checked_mul(8)? / 7;
    adjusted.checked_next_power_of_two()
}

/// The capacity of a table with `bucket_mask + 1` buckets (load factor 7/8).
#[inline]
fn bucket_mask_to_capacity(bucket_mask: usize) -> usize {
    if bucket_mask < 8 {
        bucket_mask
    } else {
        ((bucket_mask + 1) / 8) * 7
    }
}

// ---------------------------------------------------------------------------
// The table

/// The hash-independent storage of a `HashMap`: `buckets` slots of `(K, V)`
/// followed by `buckets + GROUP` control bytes (the last GROUP mirror the
/// first ones, so that a group can be loaded at any bucket), in one
/// allocation. An unallocated table has a null `ctrl`.
pub(crate) struct RawTable<K, V> {
    ctrl: *mut u8,
    slots: *mut (K, V),
    bucket_mask: usize,
    items: usize,
    growth_left: usize,
    marker: PhantomData<(K, V)>,
}

impl<K, V> RawTable<K, V> {
    #[inline]
    const fn new() -> RawTable<K, V> {
        RawTable {
            ctrl: 0usize as *mut u8,
            slots: 0usize as *mut (K, V),
            bucket_mask: 0,
            items: 0,
            growth_left: 0,
            marker: PhantomData,
        }
    }

    #[inline]
    fn align() -> usize {
        let a = mem::align_of::<(K, V)>();
        if a < 8 { 8 } else { a }
    }

    #[inline]
    fn alloc_size(buckets: usize) -> usize {
        match buckets.checked_mul(mem::size_of::<(K, V)>()) {
            Some(s) if s <= isize::MAX as usize - (buckets + GROUP) => s + buckets + GROUP,
            _ => crate::alloc::capacity_overflow(),
        }
    }

    /// A table with `buckets` (a power of two >= 4) empty buckets.
    fn with_buckets(buckets: usize) -> RawTable<K, V> {
        let size = Self::alloc_size(buckets);
        unsafe {
            let base = intrinsics::alloc(size, Self::align());
            let ctrl = intrinsics::ptr_add_mut(base, buckets * mem::size_of::<(K, V)>());
            intrinsics::write_bytes(ctrl, EMPTY, buckets + GROUP);
            RawTable {
                ctrl,
                slots: base as *mut (K, V),
                bucket_mask: buckets - 1,
                items: 0,
                growth_left: bucket_mask_to_capacity(buckets - 1),
                marker: PhantomData,
            }
        }
    }

    fn with_capacity(capacity: usize) -> RawTable<K, V> {
        if capacity == 0 {
            return RawTable::new();
        }
        match capacity_to_buckets(capacity, mem::size_of::<(K, V)>()) {
            Some(b) => RawTable::with_buckets(b),
            None => crate::alloc::capacity_overflow(),
        }
    }

    #[inline]
    fn buckets(&self) -> usize {
        if self.ctrl.is_null() { 0 } else { self.bucket_mask + 1 }
    }

    #[inline]
    fn capacity(&self) -> usize {
        self.items + self.growth_left
    }

    #[inline(always)]
    unsafe fn slot(&self, i: usize) -> *mut (K, V) {
        intrinsics::ptr_add_mut(self.slots, i)
    }

    #[inline(always)]
    unsafe fn ctrl_at(&self, i: usize) -> u8 {
        *intrinsics::ptr_add_mut(self.ctrl, i)
    }

    /// Sets a control byte (and its mirror).
    #[inline(always)]
    unsafe fn set_ctrl(&mut self, i: usize, c: u8) {
        let i2 = (i.wrapping_sub(GROUP) & self.bucket_mask) + GROUP;
        *intrinsics::ptr_add_mut(self.ctrl, i) = c;
        *intrinsics::ptr_add_mut(self.ctrl, i2) = c;
    }

    /// Frees the allocation (without dropping the elements).
    unsafe fn free_buckets(&mut self) {
        if !self.ctrl.is_null() {
            let buckets = self.bucket_mask + 1;
            intrinsics::dealloc(self.slots as *mut u8, Self::alloc_size(buckets), Self::align());
        }
    }

    /// Drops every element (leaving the control bytes as they are).
    unsafe fn drop_elements(&mut self) {
        if mem::needs_drop::<(K, V)>() && self.items != 0 {
            let mut it = self.raw_iter();
            while let Some(i) = it.next_index() {
                intrinsics::drop_in_place(self.slot(i));
            }
        }
    }

    /// Finds the bucket holding `key`.
    #[inline]
    fn find<Q: ?Sized + Eq>(&self, hash: u64, key: &Q) -> Option<usize>
    where
        K: Borrow<Q>,
    {
        if self.ctrl.is_null() {
            return None;
        }
        let tag = h2(hash);
        let mask = self.bucket_mask;
        let mut pos = (hash as usize) & mask;
        let mut stride = 0;
        unsafe {
            loop {
                let g = load_group(intrinsics::ptr_add(self.ctrl as *const u8, pos));
                let mut m = match_byte(g, tag);
                while m != 0 {
                    let i = (pos + lowest(m)) & mask;
                    if (*self.slot(i)).0.borrow() == key {
                        return Some(i);
                    }
                    m &= m - 1;
                }
                if match_empty(g) != 0 {
                    return None;
                }
                stride += GROUP;
                pos = (pos + stride) & mask;
            }
        }
    }

    /// The first EMPTY or DELETED bucket of the probe sequence of `hash`
    /// (the table is allocated and has room).
    #[inline]
    unsafe fn find_insert_slot(&self, hash: u64) -> usize {
        let mask = self.bucket_mask;
        let mut pos = (hash as usize) & mask;
        let mut stride = 0;
        loop {
            let g = load_group(intrinsics::ptr_add(self.ctrl as *const u8, pos));
            let m = match_empty_or_deleted(g);
            if m != 0 {
                let mut i = (pos + lowest(m)) & mask;
                if is_full(self.ctrl_at(i)) {
                    // (a table smaller than a group matched one of the
                    // EMPTY bytes past its end)
                    i = lowest(match_empty_or_deleted(load_group(self.ctrl as *const u8)));
                }
                return i;
            }
            stride += GROUP;
            pos = (pos + stride) & mask;
        }
    }

    /// Inserts a new element (the table has room: growth_left > 0, or the
    /// slot found is a DELETED one).
    #[inline]
    unsafe fn insert_no_grow(&mut self, hash: u64, key: K, value: V) -> usize {
        let i = self.find_insert_slot(hash);
        let old = self.ctrl_at(i);
        if old == EMPTY {
            self.growth_left -= 1;
        }
        self.set_ctrl(i, h2(hash));
        intrinsics::write(self.slot(i), (key, value));
        self.items += 1;
        i
    }

    /// Marks bucket `i` as free (its element was moved out or is dropped
    /// by the caller): EMPTY when no probe sequence can have gone past it.
    #[inline]
    unsafe fn erase(&mut self, i: usize) {
        let before = i.wrapping_sub(GROUP) & self.bucket_mask;
        let empty_before = match_empty(load_group(intrinsics::ptr_add(self.ctrl as *const u8, before)));
        let empty_after = match_empty(load_group(intrinsics::ptr_add(self.ctrl as *const u8, i)));
        let c = if (empty_before.leading_zeros() / 8 + empty_after.trailing_zeros() / 8) as usize >= GROUP {
            DELETED
        } else {
            self.growth_left += 1;
            EMPTY
        };
        self.set_ctrl(i, c);
        self.items -= 1;
    }

    /// Makes room for `additional` more elements.
    #[inline]
    fn reserve<H: Fn(&K) -> u64>(&mut self, additional: usize, hasher: &H) {
        if additional > self.growth_left {
            self.reserve_rehash(additional, hasher);
        }
    }

    #[inline(never)]
    fn reserve_rehash<H: Fn(&K) -> u64>(&mut self, additional: usize, hasher: &H) {
        let new_items = match self.items.checked_add(additional) {
            Some(n) => n,
            None => crate::alloc::capacity_overflow(),
        };
        let full_capacity = bucket_mask_to_capacity(self.bucket_mask);
        if !self.ctrl.is_null() && new_items <= full_capacity / 2 {
            // enough room once the tombstones are gone
            let b = self.buckets();
            self.rebuild(b, hasher);
        } else {
            let cap = if new_items > full_capacity + 1 { new_items } else { full_capacity + 1 };
            match capacity_to_buckets(cap, mem::size_of::<(K, V)>()) {
                Some(b) => self.rebuild(b, hasher),
                None => crate::alloc::capacity_overflow(),
            }
        }
    }

    /// Moves every element into a new table of `buckets` buckets.
    fn rebuild<H: Fn(&K) -> u64>(&mut self, buckets: usize, hasher: &H) {
        let mut new = RawTable::<K, V>::with_buckets(buckets);
        unsafe {
            if self.items != 0 {
                let mut it = self.raw_iter();
                while let Some(i) = it.next_index() {
                    let src = self.slot(i);
                    let hash = hasher(&(*src).0);
                    let j = new.find_insert_slot(hash);
                    new.set_ctrl(j, h2(hash));
                    intrinsics::copy_nonoverlapping(src as *const (K, V), new.slot(j), 1);
                }
            }
            new.items = self.items;
            new.growth_left -= self.items;
            mem::swap(self, &mut new);
            // the old table: its elements were moved
            new.free_buckets();
            mem::forget(new);
        }
    }

    /// Shrinks the table to the capacity for max(len, min_size) elements.
    fn shrink_to<H: Fn(&K) -> u64>(&mut self, min_size: usize, hasher: &H) {
        let min_size = if self.items > min_size { self.items } else { min_size };
        if min_size == 0 {
            unsafe {
                self.drop_elements();
                self.free_buckets();
            }
            self.ctrl = 0usize as *mut u8;
            self.slots = 0usize as *mut (K, V);
            self.bucket_mask = 0;
            self.items = 0;
            self.growth_left = 0;
            return;
        }
        if let Some(b) = capacity_to_buckets(min_size, mem::size_of::<(K, V)>()) {
            if b < self.buckets() {
                self.rebuild(b, hasher);
            }
        }
    }

    /// Removes every element (keeping the allocation).
    fn clear(&mut self) {
        if self.ctrl.is_null() {
            return;
        }
        unsafe {
            self.drop_elements();
            self.clear_ctrl();
        }
    }

    /// Marks every bucket EMPTY (the elements are gone).
    unsafe fn clear_ctrl(&mut self) {
        if !self.ctrl.is_null() {
            intrinsics::write_bytes(self.ctrl, EMPTY, self.bucket_mask + 1 + GROUP);
        }
        self.items = 0;
        self.growth_left = bucket_mask_to_capacity(self.bucket_mask);
    }

    #[inline]
    fn raw_iter(&self) -> RawIter {
        if self.items == 0 {
            return RawIter { ctrl: self.ctrl as *const u8, base: 0, bits: 0, left: 0 };
        }
        let bits = unsafe { match_full(load_group(self.ctrl as *const u8)) };
        RawIter { ctrl: self.ctrl as *const u8, base: 0, bits, left: self.items }
    }
}

impl<K, V> Drop for RawTable<K, V> {
    fn drop(&mut self) {
        unsafe {
            self.drop_elements();
            self.free_buckets();
        }
    }
}

impl<K: Clone, V: Clone> Clone for RawTable<K, V> {
    fn clone(&self) -> Self {
        if self.ctrl.is_null() {
            return RawTable::new();
        }
        let buckets = self.bucket_mask + 1;
        let mut new = RawTable::<K, V>::with_buckets(buckets);
        unsafe {
            // same layout (and so the same iteration order); a bucket
            // becomes full in the copy once its element is cloned
            let mut i = 0;
            while i < buckets {
                let c = self.ctrl_at(i);
                if is_full(c) {
                    let src = &*self.slot(i);
                    intrinsics::write(new.slot(i), (src.0.clone(), src.1.clone()));
                    new.items += 1;
                }
                if c != EMPTY {
                    new.set_ctrl(i, c);
                }
                i += 1;
            }
        }
        new.growth_left = self.growth_left;
        new
    }
}

/// Iterates over the indices of the full buckets (a group at a time).
#[derive(Clone, Copy)]
struct RawIter {
    ctrl: *const u8,
    base: usize,
    bits: u64,
    left: usize,
}

impl RawIter {
    #[inline(always)]
    fn next_index(&mut self) -> Option<usize> {
        if self.left == 0 {
            return None;
        }
        loop {
            if self.bits != 0 {
                let i = self.base + lowest(self.bits);
                self.bits &= self.bits - 1;
                self.left -= 1;
                return Some(i);
            }
            self.base += GROUP;
            self.bits = unsafe { match_full(load_group(intrinsics::ptr_add(self.ctrl, self.base))) };
        }
    }
}

/// A hash map.
pub struct HashMap<K, V, S = RandomState> {
    table: RawTable<K, V>,
    hash_builder: S,
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
        self.table.capacity()
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
        Iter { raw: self.table.raw_iter(), slots: self.table.slots as *const (K, V), marker: PhantomData }
    }

    /// An iterator visiting all key-value pairs in arbitrary order, with mutable references to the values.
    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        IterMut { raw: self.table.raw_iter(), slots: self.table.slots, marker: PhantomData }
    }

    /// Returns the number of elements in the map.
    #[inline]
    pub fn len(&self) -> usize {
        self.table.items
    }

    /// Returns `true` if the map contains no elements.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.table.items == 0
    }

    /// Clears the map, returning all key-value pairs as an iterator.
    #[inline]
    pub fn drain(&mut self) -> Drain<'_, K, V> {
        // the map is left empty (even if the iterator is leaked); the table
        // goes back to it, emptied, when the iterator is dropped
        let table = mem::replace(&mut self.table, RawTable::new());
        let raw = table.raw_iter();
        Drain { table, raw, orig: &mut self.table }
    }

    /// Creates an iterator which uses a closure to determine if an element should be removed.
    pub fn extract_if<F>(&mut self, pred: F) -> vec::IntoIter<(K, V)>
    where
        F: FnMut(&K, &mut V) -> bool,
    {
        let mut pred = pred;
        let mut out = Vec::new();
        unsafe {
            let mut it = self.table.raw_iter();
            while let Some(i) = it.next_index() {
                let p = self.table.slot(i);
                if pred(&(*p).0, &mut (*p).1) {
                    self.table.erase(i);
                    out.push(intrinsics::read(p as *const (K, V)));
                }
            }
        }
        out.into_iter()
    }

    /// Retains only the elements specified by the predicate.
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&K, &mut V) -> bool,
    {
        let mut f = f;
        unsafe {
            let mut it = self.table.raw_iter();
            while let Some(i) = it.next_index() {
                let p = self.table.slot(i);
                if !f(&(*p).0, &mut (*p).1) {
                    self.table.erase(i);
                    intrinsics::drop_in_place(p);
                }
            }
        }
    }

    /// Clears the map, removing all key-value pairs.
    #[inline]
    pub fn clear(&mut self) {
        self.table.clear();
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
        let hb = &self.hash_builder;
        self.table.reserve(additional, &|k: &K| hb.hash_one(k));
    }

    /// Tries to reserve capacity for at least `additional` more elements.
    pub fn try_reserve(&mut self, additional: usize) -> Result<(), crate::collections::TryReserveError> {
        if additional > self.table.growth_left {
            let ok = match self.table.items.checked_add(additional) {
                Some(n) => match capacity_to_buckets(n, mem::size_of::<(K, V)>()) {
                    Some(b) => b.checked_mul(mem::size_of::<(K, V)>()).map_or(false, |s| s <= isize::MAX as usize / 2),
                    None => false,
                },
                None => false,
            };
            if !ok {
                return Err(crate::collections::TryReserveError {
                    kind: crate::collections::TryReserveErrorKind::CapacityOverflow,
                });
            }
        }
        self.reserve(additional);
        Ok(())
    }

    /// Shrinks the capacity of the map as much as possible.
    pub fn shrink_to_fit(&mut self) {
        let hb = &self.hash_builder;
        self.table.shrink_to(0, &|k: &K| hb.hash_one(k));
    }

    /// Shrinks the capacity of the map with a lower limit.
    pub fn shrink_to(&mut self, min_capacity: usize) {
        let hb = &self.hash_builder;
        self.table.shrink_to(min_capacity, &|k: &K| hb.hash_one(k));
    }

    /// Gets the given key's corresponding entry in the map for in-place manipulation.
    pub fn entry(&mut self, key: K) -> Entry<'_, K, V> {
        let hash = self.make_hash(&key);
        match self.table.find(hash, &key) {
            Some(idx) => Entry::Occupied(OccupiedEntry { map: &mut self.table, idx, key: Some(key) }),
            None => {
                // (as in the real library: room for the insertion is made
                // now, the entry types do not know the hasher)
                self.reserve(1);
                Entry::Vacant(VacantEntry { map: &mut self.table, hash, key })
            }
        }
    }

    /// Returns a reference to the value corresponding to the key.
    #[inline]
    pub fn get<Q: ?Sized>(&self, k: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        if self.table.items == 0 {
            return None;
        }
        match self.table.find(self.make_hash(k), k) {
            Some(idx) => Some(unsafe { &(*self.table.slot(idx)).1 }),
            None => None,
        }
    }

    /// Returns the key-value pair corresponding to the supplied key.
    #[inline]
    pub fn get_key_value<Q: ?Sized>(&self, k: &Q) -> Option<(&K, &V)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        if self.table.items == 0 {
            return None;
        }
        match self.table.find(self.make_hash(k), k) {
            Some(idx) => {
                let b = unsafe { &*self.table.slot(idx) };
                Some((&b.0, &b.1))
            }
            None => None,
        }
    }

    /// Returns `true` if the map contains a value for the specified key.
    #[inline]
    pub fn contains_key<Q: ?Sized>(&self, k: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        if self.table.items == 0 {
            return false;
        }
        self.table.find(self.make_hash(k), k).is_some()
    }

    /// Returns a mutable reference to the value corresponding to the key.
    #[inline]
    pub fn get_mut<Q: ?Sized>(&mut self, k: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        if self.table.items == 0 {
            return None;
        }
        match self.table.find(self.make_hash(k), k) {
            Some(idx) => Some(unsafe { &mut (*self.table.slot(idx)).1 }),
            None => None,
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
            if let Some(idx) = self.table.find(self.make_hash(ks[i]), ks[i]) {
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
        let base = self.table.slots;
        crate::array::from_fn(|i| {
            if idxs[i] == usize::MAX {
                None
            } else {
                Some(unsafe { &mut (*base.add(idxs[i])).1 })
            }
        })
    }

    /// Inserts a key-value pair into the map, returning the old value if the key was present.
    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        let hash = self.make_hash(&k);
        // (as in the real library, room for one more element is made
        // first, even when the key turns out to be present)
        self.reserve(1);
        match self.table.find(hash, &k) {
            Some(idx) => Some(mem::replace(unsafe { &mut (*self.table.slot(idx)).1 }, v)),
            None => {
                unsafe { self.table.insert_no_grow(hash, k, v) };
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
            Some(idx) => idx,
            None => {
                self.reserve(1);
                unsafe { self.table.insert_no_grow(hash, key, value) }
            }
        };
        unsafe { &(*self.table.slot(idx)).0 }
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
        if self.table.items == 0 {
            return None;
        }
        match self.table.find(self.make_hash(k), k) {
            Some(idx) => unsafe {
                self.table.erase(idx);
                Some(intrinsics::read(self.table.slot(idx) as *const (K, V)))
            },
            None => None,
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
        let b = unsafe { &*self.entry.map.slot(self.entry.idx) };
        f.debug_struct("OccupiedError")
            .field("key", &b.0)
            .field("old_value", &b.1)
            .field("new_value", &self.value)
            .finish()
    }
}

impl<'a, K: fmt::Debug, V: fmt::Debug> fmt::Display for OccupiedError<'a, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let b = unsafe { &*self.entry.map.slot(self.entry.idx) };
        write!(
            f,
            "failed to insert {:?}, key {:?} already exists with value {:?}",
            self.value, b.0, b.1,
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
        let HashMap { table, .. } = self;
        let raw = table.raw_iter();
        IntoIter { table, raw }
    }
}

// ---------------------------------------------------------------------------
// Iterators

/// An iterator over the entries of a `HashMap`.
pub struct Iter<'a, K: 'a, V: 'a> {
    raw: RawIter,
    slots: *const (K, V),
    marker: PhantomData<&'a (K, V)>,
}

impl<'a, K, V> Clone for Iter<'a, K, V> {
    fn clone(&self) -> Self {
        Iter { raw: self.raw, slots: self.slots, marker: PhantomData }
    }
}

impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);
    #[inline]
    fn next(&mut self) -> Option<(&'a K, &'a V)> {
        match self.raw.next_index() {
            Some(i) => unsafe {
                let b = &*intrinsics::ptr_add(self.slots, i);
                Some((&b.0, &b.1))
            },
            None => None,
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.raw.left, Some(self.raw.left))
    }
    #[inline]
    fn count(self) -> usize {
        self.raw.left
    }
}

impl<'a, K, V> ExactSizeIterator for Iter<'a, K, V> {
    fn len(&self) -> usize {
        self.raw.left
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
    raw: RawIter,
    slots: *mut (K, V),
    marker: PhantomData<&'a mut (K, V)>,
}

impl<'a, K, V> Iterator for IterMut<'a, K, V> {
    type Item = (&'a K, &'a mut V);
    #[inline]
    fn next(&mut self) -> Option<(&'a K, &'a mut V)> {
        match self.raw.next_index() {
            Some(i) => unsafe {
                let b = &mut *intrinsics::ptr_add_mut(self.slots, i);
                Some((&b.0, &mut b.1))
            },
            None => None,
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.raw.left, Some(self.raw.left))
    }
}

impl<'a, K, V> ExactSizeIterator for IterMut<'a, K, V> {
    fn len(&self) -> usize {
        self.raw.left
    }
}

impl<'a, K, V> FusedIterator for IterMut<'a, K, V> {}

/// An owning iterator over the entries of a `HashMap`.
pub struct IntoIter<K, V> {
    // (`table.items` counts the elements not yet yielded)
    table: RawTable<K, V>,
    raw: RawIter,
}

impl<K, V> Iterator for IntoIter<K, V> {
    type Item = (K, V);
    #[inline]
    fn next(&mut self) -> Option<(K, V)> {
        match self.raw.next_index() {
            Some(i) => unsafe {
                self.table.set_ctrl(i, DELETED);
                self.table.items -= 1;
                Some(intrinsics::read(self.table.slot(i) as *const (K, V)))
            },
            None => None,
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.raw.left, Some(self.raw.left))
    }
}

impl<K, V> ExactSizeIterator for IntoIter<K, V> {
    fn len(&self) -> usize {
        self.raw.left
    }
}

impl<K, V> FusedIterator for IntoIter<K, V> {}

/// A draining iterator over the entries of a `HashMap`.
pub struct Drain<'a, K: 'a, V: 'a> {
    // the map's table (emptied when the iterator is dropped, then put back)
    table: RawTable<K, V>,
    raw: RawIter,
    orig: &'a mut RawTable<K, V>,
}

impl<'a, K, V> Iterator for Drain<'a, K, V> {
    type Item = (K, V);
    #[inline]
    fn next(&mut self) -> Option<(K, V)> {
        match self.raw.next_index() {
            Some(i) => unsafe {
                self.table.set_ctrl(i, DELETED);
                self.table.items -= 1;
                Some(intrinsics::read(self.table.slot(i) as *const (K, V)))
            },
            None => None,
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.raw.left, Some(self.raw.left))
    }
}

impl<'a, K, V> Drop for Drain<'a, K, V> {
    fn drop(&mut self) {
        unsafe {
            // drop what was not yielded, then hand the empty table back
            self.table.drop_elements();
            self.table.clear_ctrl();
            mem::swap(&mut self.table, self.orig);
        }
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
        unsafe { &(*self.map.slot(self.idx)).0 }
    }

    /// Take the ownership of the key and value from the map.
    pub fn remove_entry(self) -> (K, V) {
        unsafe {
            self.map.erase(self.idx);
            intrinsics::read(self.map.slot(self.idx) as *const (K, V))
        }
    }

    /// Gets a reference to the value in the entry.
    #[inline]
    pub fn get(&self) -> &V {
        unsafe { &(*self.map.slot(self.idx)).1 }
    }

    /// Gets a mutable reference to the value in the entry.
    #[inline]
    pub fn get_mut(&mut self) -> &mut V {
        unsafe { &mut (*self.map.slot(self.idx)).1 }
    }

    /// Converts the `OccupiedEntry` into a mutable reference to the value in the entry with a lifetime bound to the map itself.
    #[inline]
    pub fn into_mut(self) -> &'a mut V {
        unsafe { &mut (*self.map.slot(self.idx)).1 }
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
        let b = unsafe { &mut *self.map.slot(idx) };
        let old_key = mem::replace(&mut b.0, new_key);
        let old_value = mem::replace(&mut b.1, value);
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
        unsafe {
            let idx = map.insert_no_grow(self.hash, self.key, value);
            &mut (*map.slot(idx)).1
        }
    }

    /// Sets the value of the entry with the `VacantEntry`'s key, and returns an `OccupiedEntry`.
    #[inline]
    pub fn insert_entry(self, value: V) -> OccupiedEntry<'a, K, V> {
        let map = self.map;
        let idx = unsafe { map.insert_no_grow(self.hash, self.key, value) };
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
        let b = unsafe { &*self.map.slot(self.idx) };
        f.debug_struct("OccupiedEntry").field("key", &b.0).field("value", &b.1).finish_non_exhaustive()
    }
}

impl<'a, K: fmt::Debug, V> fmt::Debug for VacantEntry<'a, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("VacantEntry").field(&self.key).finish()
    }
}
