// A const-generic ring buffer over [MaybeUninit<T>; N] with drop tracking, and a raw
// pointer slice iterator (ptr/end with PhantomData lifetimes, double-ended, exact-size,
// ZST-aware) plus IterMut and an owning IntoIter.
use std::marker::PhantomData;
use std::mem::{self, MaybeUninit};
use std::ptr;

struct Ring<T, const N: usize> {
    buf: [MaybeUninit<T>; N],
    head: usize,
    len: usize,
}

impl<T, const N: usize> Ring<T, N> {
    fn new() -> Self {
        Ring { buf: [const { MaybeUninit::uninit() }; N], head: 0, len: 0 }
    }
    fn push(&mut self, v: T) -> Option<T> {
        let idx = (self.head + self.len) % N;
        if self.len == N {
            // overwrite the oldest
            let old = unsafe { self.buf[self.head].assume_init_read() };
            self.buf[self.head].write(v);
            self.head = (self.head + 1) % N;
            Some(old)
        } else {
            self.buf[idx].write(v);
            self.len += 1;
            None
        }
    }
    fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        let v = unsafe { self.buf[self.head].assume_init_read() };
        self.head = (self.head + 1) % N;
        self.len -= 1;
        Some(v)
    }
    fn get(&self, i: usize) -> Option<&T> {
        (i < self.len).then(|| unsafe { self.buf[(self.head + i) % N].assume_init_ref() })
    }
}
impl<T, const N: usize> Drop for Ring<T, N> {
    fn drop(&mut self) {
        while self.pop().is_some() {}
    }
}

struct Iter<'a, T> {
    ptr: *const T,
    end: *const T,
    _m: PhantomData<&'a T>,
}
impl<'a, T> Iter<'a, T> {
    fn new(s: &'a [T]) -> Self {
        let ptr = s.as_ptr();
        let end = if mem::size_of::<T>() == 0 {
            (ptr as usize + s.len()) as *const T
        } else {
            unsafe { ptr.add(s.len()) }
        };
        Iter { ptr, end, _m: PhantomData }
    }
}
impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<&'a T> {
        if self.ptr == self.end {
            return None;
        }
        unsafe {
            if mem::size_of::<T>() == 0 {
                self.ptr = (self.ptr as usize + 1) as *const T;
                Some(&*ptr::NonNull::<T>::dangling().as_ptr())
            } else {
                let r = &*self.ptr;
                self.ptr = self.ptr.add(1);
                Some(r)
            }
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = if mem::size_of::<T>() == 0 {
            self.end as usize - self.ptr as usize
        } else {
            unsafe { self.end.offset_from(self.ptr) as usize }
        };
        (n, Some(n))
    }
}
impl<'a, T> DoubleEndedIterator for Iter<'a, T> {
    fn next_back(&mut self) -> Option<&'a T> {
        if self.ptr == self.end {
            return None;
        }
        unsafe {
            if mem::size_of::<T>() == 0 {
                self.end = (self.end as usize - 1) as *const T;
                Some(&*ptr::NonNull::<T>::dangling().as_ptr())
            } else {
                self.end = self.end.sub(1);
                Some(&*self.end)
            }
        }
    }
}
impl<T> ExactSizeIterator for Iter<'_, T> {}

struct IterMut<'a, T> {
    ptr: *mut T,
    left: usize,
    _m: PhantomData<&'a mut T>,
}
impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;
    fn next(&mut self) -> Option<&'a mut T> {
        if self.left == 0 {
            return None;
        }
        self.left -= 1;
        unsafe {
            let r = &mut *self.ptr;
            self.ptr = self.ptr.add(1);
            Some(r)
        }
    }
}
fn iter_mut<T>(s: &mut [T]) -> IterMut<'_, T> {
    IterMut { ptr: s.as_mut_ptr(), left: s.len(), _m: PhantomData }
}

struct IntoIter<T> {
    buf: *mut T,
    cap: usize,
    pos: usize,
    len: usize,
}
impl<T> IntoIter<T> {
    fn new(v: Vec<T>) -> Self {
        let mut v = mem::ManuallyDrop::new(v);
        IntoIter { buf: v.as_mut_ptr(), cap: v.capacity(), pos: 0, len: v.len() }
    }
}
impl<T> Iterator for IntoIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        if self.pos == self.len {
            return None;
        }
        let v = unsafe { ptr::read(self.buf.add(self.pos)) };
        self.pos += 1;
        Some(v)
    }
}
impl<T> Drop for IntoIter<T> {
    fn drop(&mut self) {
        unsafe {
            let rest = ptr::slice_from_raw_parts_mut(self.buf.add(self.pos), self.len - self.pos);
            ptr::drop_in_place(rest);
            drop(Vec::from_raw_parts(self.buf, 0, self.cap));
        }
    }
}

#[derive(Debug)]
struct Tracked(u32);
impl Drop for Tracked {
    fn drop(&mut self) {
        println!("drop Tracked({})", self.0);
    }
}

fn main() {
    let mut r: Ring<Tracked, 3> = Ring::new();
    for i in 1..=5 {
        if let Some(old) = r.push(Tracked(i)) {
            println!("evicted {:?}", old);
        }
    }
    println!("get0 {:?} get2 {:?} get3 {:?}", r.get(0), r.get(2), r.get(3));
    println!("pop {:?}", r.pop());
    r.push(Tracked(6));
    println!("dropping ring with {}", r.len);
    drop(r);
    let mut rs: Ring<String, 2> = Ring::new();
    rs.push("a".into());
    println!("{:?} {:?}", rs.pop(), rs.pop());

    let data = [10, 20, 30, 40, 50];
    let it = Iter::new(&data);
    println!("len {}", it.len());
    let v: Vec<_> = Iter::new(&data).rev().collect();
    println!("{:?}", v);
    let mut it = Iter::new(&data);
    println!("{:?} {:?} {:?} rem {}", it.next(), it.next_back(), it.next(), it.len());
    let zs = [(), (), ()];
    let zi = Iter::new(&zs);
    println!("zst len {} count {}", zi.len(), Iter::new(&zs).rev().count());
    let mut m = vec![1, 2, 3];
    for x in iter_mut(&mut m) {
        *x *= 7;
    }
    println!("{:?}", m);
    let tracked = vec![Tracked(100), Tracked(101), Tracked(102), Tracked(103)];
    let mut ii = IntoIter::new(tracked);
    println!("took {:?}", ii.next().map(|t| t.0));
    println!("took {:?}", ii.next().map(|t| t.0));
    println!("dropping iterator");
    drop(ii);
    let strings = IntoIter::new(vec!["x".to_string(), "y".to_string()]);
    println!("{}", strings.collect::<Vec<_>>().join("+"));
}
