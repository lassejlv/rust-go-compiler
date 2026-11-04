// Hand-rolled collections with unsafe internals: an inline SmallVec using
// MaybeUninit, a NonNull doubly-linked list with a cursor, a fixed ring
// buffer, a bitset with set algebra via operator traits, and a generational
// arena -- all checked for leaks/double drops with a counting Drop type.
use std::cell::Cell;
use std::fmt;
use std::mem::MaybeUninit;
use std::ops::{BitAnd, BitOr, Deref, DerefMut, Index, IndexMut, Not, Sub};
use std::ptr::{self, NonNull};

thread_local! {
    static LIVE: Cell<i64> = Cell::new(0);
    static DROPPED: Cell<u64> = Cell::new(0);
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Tracked(u32);
impl Tracked {
    fn new(v: u32) -> Self {
        LIVE.with(|l| l.set(l.get() + 1));
        Tracked(v)
    }
}
impl Clone for Tracked {
    fn clone(&self) -> Self {
        Tracked::new(self.0)
    }
}
impl Drop for Tracked {
    fn drop(&mut self) {
        LIVE.with(|l| l.set(l.get() - 1));
        DROPPED.with(|d| d.set(d.get() + 1));
    }
}
fn live() -> i64 {
    LIVE.with(|l| l.get())
}

// ---------------- SmallVec ----------------
enum Storage<T, const N: usize> {
    Inline([MaybeUninit<T>; N], usize),
    Heap(Vec<T>),
}

struct SmallVec<T, const N: usize> {
    s: Storage<T, N>,
}

impl<T, const N: usize> SmallVec<T, N> {
    fn new() -> Self {
        SmallVec { s: Storage::Inline([const { MaybeUninit::uninit() }; N], 0) }
    }
    fn spilled(&self) -> bool {
        matches!(self.s, Storage::Heap(_))
    }
    fn push(&mut self, v: T) {
        match &mut self.s {
            Storage::Inline(arr, len) if *len < N => {
                arr[*len].write(v);
                *len += 1;
            }
            Storage::Inline(arr, len) => {
                let mut vec = Vec::with_capacity(N * 2);
                for slot in arr.iter_mut().take(*len) {
                    // move each initialized element out exactly once
                    vec.push(unsafe { slot.assume_init_read() });
                }
                *len = 0;
                vec.push(v);
                self.s = Storage::Heap(vec);
            }
            Storage::Heap(vec) => vec.push(v),
        }
    }
    fn pop(&mut self) -> Option<T> {
        match &mut self.s {
            Storage::Inline(_, 0) => None,
            Storage::Inline(arr, len) => {
                *len -= 1;
                Some(unsafe { arr[*len].assume_init_read() })
            }
            Storage::Heap(v) => v.pop(),
        }
    }
    fn insert(&mut self, idx: usize, v: T) {
        assert!(idx <= self.len(), "insert index {} out of bounds", idx);
        match &mut self.s {
            Storage::Inline(arr, len) if *len < N => unsafe {
                let p = arr.as_mut_ptr() as *mut T;
                ptr::copy(p.add(idx), p.add(idx + 1), *len - idx);
                ptr::write(p.add(idx), v);
                *len += 1;
            },
            _ => {
                self.push(v);
                let l = self.len();
                self[idx..].rotate_right(1);
                let _ = l;
            }
        }
    }
    fn remove(&mut self, idx: usize) -> T {
        let len = self.len();
        assert!(idx < len);
        match &mut self.s {
            Storage::Inline(arr, l) => unsafe {
                let p = arr.as_mut_ptr() as *mut T;
                let out = ptr::read(p.add(idx));
                ptr::copy(p.add(idx + 1), p.add(idx), *l - idx - 1);
                *l -= 1;
                out
            },
            Storage::Heap(v) => v.remove(idx),
        }
    }
}

impl<T, const N: usize> Deref for SmallVec<T, N> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        match &self.s {
            Storage::Inline(arr, len) => unsafe { std::slice::from_raw_parts(arr.as_ptr() as *const T, *len) },
            Storage::Heap(v) => v,
        }
    }
}
impl<T, const N: usize> DerefMut for SmallVec<T, N> {
    fn deref_mut(&mut self) -> &mut [T] {
        match &mut self.s {
            Storage::Inline(arr, len) => unsafe { std::slice::from_raw_parts_mut(arr.as_mut_ptr() as *mut T, *len) },
            Storage::Heap(v) => v,
        }
    }
}
impl<T, const N: usize> Drop for SmallVec<T, N> {
    fn drop(&mut self) {
        if let Storage::Inline(arr, len) = &mut self.s {
            for slot in &mut arr[..*len] {
                unsafe { slot.assume_init_drop() };
            }
        }
    }
}
impl<T: fmt::Debug, const N: usize> fmt::Debug for SmallVec<T, N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}
impl<T, const N: usize> FromIterator<T> for SmallVec<T, N> {
    fn from_iter<I: IntoIterator<Item = T>>(it: I) -> Self {
        let mut s = SmallVec::new();
        for x in it {
            s.push(x);
        }
        s
    }
}

// ---------------- linked list ----------------
struct Node<T> {
    val: T,
    prev: Option<NonNull<Node<T>>>,
    next: Option<NonNull<Node<T>>>,
}

struct List<T> {
    head: Option<NonNull<Node<T>>>,
    tail: Option<NonNull<Node<T>>>,
    len: usize,
}

impl<T> List<T> {
    fn new() -> Self {
        List { head: None, tail: None, len: 0 }
    }
    fn push_back(&mut self, val: T) {
        let node = Box::new(Node { val, prev: self.tail, next: None });
        let p = NonNull::from(Box::leak(node));
        match self.tail {
            Some(mut t) => unsafe { t.as_mut().next = Some(p) },
            None => self.head = Some(p),
        }
        self.tail = Some(p);
        self.len += 1;
    }
    fn push_front(&mut self, val: T) {
        let p = NonNull::from(Box::leak(Box::new(Node { val, prev: None, next: self.head })));
        match self.head {
            Some(mut h) => unsafe { h.as_mut().prev = Some(p) },
            None => self.tail = Some(p),
        }
        self.head = Some(p);
        self.len += 1;
    }
    fn pop_front(&mut self) -> Option<T> {
        self.head.map(|h| unsafe {
            let b = Box::from_raw(h.as_ptr());
            self.head = b.next;
            match self.head {
                Some(mut n) => n.as_mut().prev = None,
                None => self.tail = None,
            }
            self.len -= 1;
            b.val
        })
    }
    fn iter(&self) -> ListIter<'_, T> {
        ListIter { cur: self.head, _l: std::marker::PhantomData }
    }
    /// Remove all nodes matching pred, returning them in order.
    fn drain_filter(&mut self, mut pred: impl FnMut(&T) -> bool) -> Vec<T> {
        let mut out = Vec::new();
        let mut cur = self.head;
        while let Some(n) = cur {
            unsafe {
                let next = (*n.as_ptr()).next;
                if pred(&(*n.as_ptr()).val) {
                    let prev = (*n.as_ptr()).prev;
                    match prev {
                        Some(mut p) => p.as_mut().next = next,
                        None => self.head = next,
                    }
                    match next {
                        Some(mut x) => x.as_mut().prev = prev,
                        None => self.tail = prev,
                    }
                    self.len -= 1;
                    out.push(Box::from_raw(n.as_ptr()).val);
                }
                cur = next;
            }
        }
        out
    }
    fn reverse(&mut self) {
        let mut cur = self.head;
        while let Some(mut n) = cur {
            unsafe {
                let node = n.as_mut();
                std::mem::swap(&mut node.prev, &mut node.next);
                cur = node.prev;
            }
        }
        std::mem::swap(&mut self.head, &mut self.tail);
    }
}

struct ListIter<'a, T> {
    cur: Option<NonNull<Node<T>>>,
    _l: std::marker::PhantomData<&'a T>,
}
impl<'a, T> Iterator for ListIter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<&'a T> {
        self.cur.map(|n| unsafe {
            let r = &*n.as_ptr();
            self.cur = r.next;
            &r.val
        })
    }
}
impl<T> Drop for List<T> {
    fn drop(&mut self) {
        while self.pop_front().is_some() {}
    }
}

// ---------------- ring buffer ----------------
struct Ring<T, const N: usize> {
    buf: [Option<T>; N],
    head: usize,
    len: usize,
}

impl<T, const N: usize> Ring<T, N> {
    fn new() -> Self {
        Ring { buf: std::array::from_fn(|_| None), head: 0, len: 0 }
    }
    /// push, evicting the oldest when full
    fn push(&mut self, v: T) -> Option<T> {
        let idx = (self.head + self.len) % N;
        if self.len == N {
            let old = self.buf[self.head].replace(v);
            self.head = (self.head + 1) % N;
            old
        } else {
            self.buf[idx] = Some(v);
            self.len += 1;
            None
        }
    }
    fn iter(&self) -> impl Iterator<Item = &T> + '_ {
        (0..self.len).map(move |i| self.buf[(self.head + i) % N].as_ref().unwrap())
    }
}

impl<T, const N: usize> Index<usize> for Ring<T, N> {
    type Output = T;
    fn index(&self, i: usize) -> &T {
        assert!(i < self.len);
        self.buf[(self.head + i) % N].as_ref().unwrap()
    }
}
impl<T, const N: usize> IndexMut<usize> for Ring<T, N> {
    fn index_mut(&mut self, i: usize) -> &mut T {
        assert!(i < self.len);
        self.buf[(self.head + i) % N].as_mut().unwrap()
    }
}

// ---------------- bitset ----------------
#[derive(Clone, PartialEq, Eq, Hash, Default)]
struct BitSet {
    words: Vec<u64>,
}

impl BitSet {
    fn with(bits: impl IntoIterator<Item = usize>) -> Self {
        let mut s = BitSet::default();
        for b in bits {
            s.insert(b);
        }
        s
    }
    fn insert(&mut self, b: usize) -> bool {
        let (w, m) = (b / 64, 1u64 << (b % 64));
        if self.words.len() <= w {
            self.words.resize(w + 1, 0);
        }
        let had = self.words[w] & m != 0;
        self.words[w] |= m;
        !had
    }
    fn contains(&self, b: usize) -> bool {
        self.words.get(b / 64).map_or(false, |w| w >> (b % 64) & 1 == 1)
    }
    fn len(&self) -> usize {
        self.words.iter().map(|w| w.count_ones() as usize).sum()
    }
    fn iter(&self) -> impl Iterator<Item = usize> + '_ {
        self.words.iter().enumerate().flat_map(|(i, &w)| {
            let mut w = w;
            std::iter::from_fn(move || {
                if w == 0 {
                    return None;
                }
                let t = w.trailing_zeros() as usize;
                w &= w - 1;
                Some(i * 64 + t)
            })
        })
    }
    fn zip_words(&self, o: &BitSet, f: impl Fn(u64, u64) -> u64) -> BitSet {
        let n = self.words.len().max(o.words.len());
        let mut words: Vec<u64> = (0..n).map(|i| f(*self.words.get(i).unwrap_or(&0), *o.words.get(i).unwrap_or(&0))).collect();
        while words.last() == Some(&0) {
            words.pop();
        }
        BitSet { words }
    }
}

impl<'a> BitOr for &'a BitSet {
    type Output = BitSet;
    fn bitor(self, o: &'a BitSet) -> BitSet {
        self.zip_words(o, |a, b| a | b)
    }
}
impl<'a> BitAnd for &'a BitSet {
    type Output = BitSet;
    fn bitand(self, o: &'a BitSet) -> BitSet {
        self.zip_words(o, |a, b| a & b)
    }
}
impl<'a> Sub for &'a BitSet {
    type Output = BitSet;
    fn sub(self, o: &'a BitSet) -> BitSet {
        self.zip_words(o, |a, b| a & !b)
    }
}
impl Not for &BitSet {
    type Output = BitSet;
    fn not(self) -> BitSet {
        // complement within the allocated words
        BitSet { words: self.words.iter().map(|w| !w).collect() }
    }
}
impl fmt::Debug for BitSet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

// ---------------- generational arena ----------------
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Idx {
    slot: u32,
    gen: u32,
}

enum Slot<T> {
    Used { gen: u32, val: T },
    Free { gen: u32, next: Option<u32> },
}

struct Arena<T> {
    slots: Vec<Slot<T>>,
    free: Option<u32>,
    len: usize,
}

impl<T> Arena<T> {
    fn new() -> Self {
        Arena { slots: vec![], free: None, len: 0 }
    }
    fn insert(&mut self, val: T) -> Idx {
        self.len += 1;
        match self.free {
            Some(i) => {
                let gen = match self.slots[i as usize] {
                    Slot::Free { gen, next } => {
                        self.free = next;
                        gen + 1
                    }
                    Slot::Used { .. } => unreachable!("free list corrupted"),
                };
                self.slots[i as usize] = Slot::Used { gen, val };
                Idx { slot: i, gen }
            }
            None => {
                self.slots.push(Slot::Used { gen: 0, val });
                Idx { slot: self.slots.len() as u32 - 1, gen: 0 }
            }
        }
    }
    fn remove(&mut self, i: Idx) -> Option<T> {
        match self.slots.get(i.slot as usize) {
            Some(Slot::Used { gen, .. }) if *gen == i.gen => {}
            _ => return None,
        }
        let old = std::mem::replace(&mut self.slots[i.slot as usize], Slot::Free { gen: i.gen, next: self.free });
        self.free = Some(i.slot);
        self.len -= 1;
        match old {
            Slot::Used { val, .. } => Some(val),
            Slot::Free { .. } => None,
        }
    }
    fn get(&self, i: Idx) -> Option<&T> {
        match self.slots.get(i.slot as usize)? {
            Slot::Used { gen, val } if *gen == i.gen => Some(val),
            _ => None,
        }
    }
    fn get_mut(&mut self, i: Idx) -> Option<&mut T> {
        match self.slots.get_mut(i.slot as usize)? {
            Slot::Used { gen, val } if *gen == i.gen => Some(val),
            _ => None,
        }
    }
}

fn main() {
    {
        let mut sv: SmallVec<Tracked, 4> = SmallVec::new();
        for i in 0..3 {
            sv.push(Tracked::new(i));
        }
        sv.insert(1, Tracked::new(100));
        println!("inline {:?} spilled={} live={}", sv, sv.spilled(), live());
        let r = sv.remove(0);
        println!("removed {:?} -> {:?}", r, sv);
        drop(r);
        sv.push(Tracked::new(7));
        sv.push(Tracked::new(8));
        sv.insert(0, Tracked::new(42));
        println!("heap {:?} spilled={} len={} live={}", sv, sv.spilled(), sv.len(), live());
        sv.sort();
        println!("sorted {:?} max {:?}", sv, sv.iter().max());
        println!("pop {:?}", sv.pop());
        let doubled: SmallVec<u32, 8> = sv.iter().map(|t| t.0 * 2).collect();
        println!("{:?} {} {:?}", doubled, doubled.spilled(), doubled.binary_search(&84));
        let mut small: SmallVec<String, 2> = SmallVec::new();
        small.push("a".into());
        println!("{:?} {:?} {:?}", small.pop(), small.pop(), small);
    }
    println!("after smallvec: live={}", live());

    {
        let mut l = List::new();
        for i in 0..6 {
            l.push_back(Tracked::new(i));
        }
        l.push_front(Tracked::new(99));
        let evens = l.drain_filter(|t| t.0 % 2 == 0);
        println!("list {:?} drained {:?} len {}", l.iter().map(|t| t.0).collect::<Vec<_>>(), evens.iter().map(|t| t.0).collect::<Vec<_>>(), l.len);
        l.reverse();
        println!("reversed {:?} front {:?}", l.iter().map(|t| t.0).collect::<Vec<_>>(), l.pop_front().map(|t| t.0));
        let mut words = List::new();
        "the quick brown fox".split(' ').for_each(|w| words.push_back(w.to_string()));
        words.reverse();
        println!("{:?}", words.iter().cloned().collect::<Vec<_>>().join(" "));
        println!("live inside={}", live());
    }
    println!("after list: live={}", live());

    let mut ring: Ring<String, 3> = Ring::new();
    let evicted: Vec<Option<String>> = ["a", "b", "c", "d", "e"].iter().map(|s| ring.push(s.to_string())).collect();
    ring[0].push('!');
    println!("ring {:?} evicted {:?} first {}", ring.iter().collect::<Vec<_>>(), evicted, ring[0]);

    let a = BitSet::with([1, 3, 5, 64, 130]);
    let b = BitSet::with((0..10).filter(|x| x % 3 == 0).chain([130, 200]));
    println!("a={:?} b={:?}", a, b);
    println!("a|b={:?} a&b={:?} a-b={:?} |a|={} contains(64)={} contains(65)={}", &a | &b, &a & &b, &a - &b, a.len(), a.contains(64), a.contains(65));
    println!("!a has {} bits, includes 0: {}", (!&a).len(), (!&a).contains(0));

    {
        let mut arena = Arena::new();
        let ids: Vec<Idx> = (0..5).map(|i| arena.insert(Tracked::new(i * 10))).collect();
        let removed = arena.remove(ids[1]).map(|t| t.0);
        let again = arena.remove(ids[1]).map(|t| t.0);
        let n = arena.insert(Tracked::new(777));
        if let Some(t) = arena.get_mut(ids[3]) {
            t.0 += 1;
        }
        println!("arena removed {:?} again {:?} new {:?} stale get {:?} fresh get {:?} len {}", removed, again, n, arena.get(ids[1]).map(|t| t.0), arena.get(n).map(|t| t.0), arena.len);
        println!("values {:?} live={}", ids.iter().filter_map(|&i| arena.get(i)).map(|t| t.0).collect::<Vec<_>>(), live());
    }
    println!("final live={} dropped={}", live(), DROPPED.with(|d| d.get()));
}
