// A SmallVec-like container: inline storage in MaybeUninit<[T; N]>, spills to the heap.
use std::fmt;
use std::mem::{ManuallyDrop, MaybeUninit};
use std::ops::{Deref, DerefMut};
use std::ptr;

union Data<T, const N: usize> {
    inline: ManuallyDrop<MaybeUninit<[T; N]>>,
    heap: (*mut T, usize), // ptr, capacity
}

struct SmallVec<T, const N: usize> {
    len: usize,
    spilled: bool,
    data: Data<T, N>,
}

impl<T, const N: usize> SmallVec<T, N> {
    fn new() -> Self {
        SmallVec { len: 0, spilled: false, data: Data { inline: ManuallyDrop::new(MaybeUninit::uninit()) } }
    }

    fn capacity(&self) -> usize {
        if self.spilled {
            unsafe { self.data.heap.1 }
        } else {
            N
        }
    }

    fn as_ptr(&self) -> *const T {
        unsafe {
            if self.spilled {
                self.data.heap.0
            } else {
                (*self.data.inline).as_ptr() as *const T
            }
        }
    }

    fn as_mut_ptr(&mut self) -> *mut T {
        unsafe {
            if self.spilled {
                self.data.heap.0
            } else {
                (*self.data.inline).as_mut_ptr() as *mut T
            }
        }
    }

    fn grow(&mut self, new_cap: usize) {
        let mut v: Vec<T> = Vec::with_capacity(new_cap);
        unsafe {
            ptr::copy_nonoverlapping(self.as_ptr(), v.as_mut_ptr(), self.len);
            if self.spilled {
                let (p, cap) = self.data.heap;
                drop(Vec::from_raw_parts(p, 0, cap));
            }
            let cap = v.capacity();
            let p = v.as_mut_ptr();
            std::mem::forget(v);
            self.data.heap = (p, cap);
        }
        self.spilled = true;
    }

    fn push(&mut self, value: T) {
        if self.len == self.capacity() {
            let nc = (self.capacity() * 2).max(1);
            self.grow(nc);
        }
        unsafe {
            ptr::write(self.as_mut_ptr().add(self.len), value);
        }
        self.len += 1;
    }

    fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        unsafe { Some(ptr::read(self.as_ptr().add(self.len))) }
    }

    fn insert(&mut self, idx: usize, value: T) {
        assert!(idx <= self.len);
        if self.len == self.capacity() {
            let nc = (self.capacity() * 2).max(1);
            self.grow(nc);
        }
        unsafe {
            let p = self.as_mut_ptr().add(idx);
            ptr::copy(p, p.add(1), self.len - idx);
            ptr::write(p, value);
        }
        self.len += 1;
    }

    fn remove(&mut self, idx: usize) -> T {
        assert!(idx < self.len);
        unsafe {
            let p = self.as_mut_ptr().add(idx);
            let v = ptr::read(p);
            ptr::copy(p.add(1), p, self.len - idx - 1);
            self.len -= 1;
            v
        }
    }

    fn spilled(&self) -> bool {
        self.spilled
    }
}

impl<T, const N: usize> Deref for SmallVec<T, N> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.as_ptr(), self.len) }
    }
}

impl<T, const N: usize> DerefMut for SmallVec<T, N> {
    fn deref_mut(&mut self) -> &mut [T] {
        unsafe { std::slice::from_raw_parts_mut(self.as_mut_ptr(), self.len) }
    }
}

impl<T, const N: usize> Drop for SmallVec<T, N> {
    fn drop(&mut self) {
        unsafe {
            ptr::drop_in_place(&mut self[..] as *mut [T]);
            if self.spilled {
                let (p, cap) = self.data.heap;
                drop(Vec::from_raw_parts(p, 0, cap));
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

struct Noisy(&'static str);
impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let mut v: SmallVec<i32, 4> = SmallVec::new();
    for i in 1..=3 {
        v.push(i * 10);
    }
    println!("{:?} len={} cap={} spilled={}", v, v.len(), v.capacity(), v.spilled());
    v.push(40);
    println!("{:?} spilled={}", v, v.spilled());
    v.push(50);
    println!("{:?} cap={} spilled={}", v, v.capacity(), v.spilled());
    v.insert(0, 5);
    v.insert(3, 25);
    println!("{:?}", v);
    println!("removed {} -> {:?}", v.remove(1), v);
    v.sort_by(|a, b| b.cmp(a));
    println!("sorted desc {:?} sum={}", v, v.iter().sum::<i32>());
    while let Some(x) = v.pop() {
        if x > 30 {
            break;
        }
    }
    println!("after pops {:?}", v);

    let s: SmallVec<String, 2> = "a bb ccc dddd".split(' ').map(String::from).collect();
    println!("{:?} spilled={}", s, s.spilled());
    let lens: Vec<usize> = s.iter().map(|x| x.len()).collect();
    println!("{:?}", lens);

    {
        let mut n: SmallVec<Noisy, 2> = SmallVec::new();
        n.push(Noisy("a"));
        n.push(Noisy("b"));
        let popped = n.pop();
        println!("popped {}", popped.as_ref().map(|x| x.0).unwrap());
        n.push(Noisy("c"));
        n.push(Noisy("d"));
        println!("spilled={} len={}", n.spilled(), n.len());
    }
    println!("end");
}
