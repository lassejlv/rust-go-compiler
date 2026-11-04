// Fat raw pointers (Box<dyn Trait>/Box<[T]>/Box<str> into_raw/from_raw), a custom Rc with
// NonNull + PhantomData, a ZST-aware vector, slice::align_to, String::from_raw_parts,
// and a Treiber stack on AtomicPtr shared by threads.
use std::alloc::{self, Layout};
use std::marker::PhantomData;
use std::mem;
use std::ptr::{self, NonNull};
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

trait Speak {
    fn speak(&self) -> String;
}
struct Dog(u8);
impl Speak for Dog {
    fn speak(&self) -> String {
        format!("woof x{}", self.0)
    }
}
impl Drop for Dog {
    fn drop(&mut self) {
        println!("drop Dog {}", self.0);
    }
}

struct MyRc<T> {
    ptr: NonNull<RcBox<T>>,
    _p: PhantomData<RcBox<T>>,
}
struct RcBox<T> {
    count: usize,
    val: T,
}
impl<T> MyRc<T> {
    fn new(val: T) -> Self {
        let b = Box::new(RcBox { count: 1, val });
        MyRc { ptr: NonNull::from(Box::leak(b)), _p: PhantomData }
    }
    fn count(&self) -> usize {
        unsafe { self.ptr.as_ref().count }
    }
}
impl<T> Clone for MyRc<T> {
    fn clone(&self) -> Self {
        unsafe { (*self.ptr.as_ptr()).count += 1 };
        MyRc { ptr: self.ptr, _p: PhantomData }
    }
}
impl<T> std::ops::Deref for MyRc<T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &self.ptr.as_ref().val }
    }
}
impl<T> Drop for MyRc<T> {
    fn drop(&mut self) {
        unsafe {
            let b = self.ptr.as_ptr();
            (*b).count -= 1;
            if (*b).count == 0 {
                drop(Box::from_raw(b));
            }
        }
    }
}

struct MiniVec<T> {
    ptr: NonNull<T>,
    cap: usize,
    len: usize,
}
impl<T> MiniVec<T> {
    fn new() -> Self {
        let cap = if mem::size_of::<T>() == 0 { usize::MAX } else { 0 };
        MiniVec { ptr: NonNull::dangling(), cap, len: 0 }
    }
    fn push(&mut self, v: T) {
        if self.len == self.cap {
            let new_cap = if self.cap == 0 { 4 } else { self.cap * 2 };
            let new_layout = Layout::array::<T>(new_cap).unwrap();
            let p = if self.cap == 0 {
                unsafe { alloc::alloc(new_layout) }
            } else {
                unsafe { alloc::realloc(self.ptr.as_ptr() as *mut u8, Layout::array::<T>(self.cap).unwrap(), new_layout.size()) }
            };
            self.ptr = NonNull::new(p as *mut T).expect("oom");
            self.cap = new_cap;
        }
        unsafe { ptr::write(self.ptr.as_ptr().add(self.len), v) };
        self.len += 1;
    }
    fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;
            unsafe { Some(ptr::read(self.ptr.as_ptr().add(self.len))) }
        }
    }
}
impl<T> Drop for MiniVec<T> {
    fn drop(&mut self) {
        while self.pop().is_some() {}
        if mem::size_of::<T>() != 0 && self.cap != 0 {
            unsafe { alloc::dealloc(self.ptr.as_ptr() as *mut u8, Layout::array::<T>(self.cap).unwrap()) };
        }
    }
}

struct Node {
    val: u32,
    next: *mut Node,
}
struct Stack {
    head: AtomicPtr<Node>,
}
impl Stack {
    fn push(&self, val: u32) {
        let n = Box::into_raw(Box::new(Node { val, next: ptr::null_mut() }));
        let mut head = self.head.load(Ordering::Relaxed);
        loop {
            unsafe { (*n).next = head };
            match self.head.compare_exchange_weak(head, n, Ordering::Release, Ordering::Relaxed) {
                Ok(_) => return,
                Err(h) => head = h,
            }
        }
    }
    fn drain(&self) -> Vec<u32> {
        let mut p = self.head.swap(ptr::null_mut(), Ordering::Acquire);
        let mut out = vec![];
        while !p.is_null() {
            let b = unsafe { Box::from_raw(p) };
            out.push(b.val);
            p = b.next;
        }
        out
    }
}

#[derive(Clone, Copy, Debug)]
struct Unit;

fn main() {
    let b: Box<dyn Speak> = Box::new(Dog(3));
    let raw: *mut dyn Speak = Box::into_raw(b);
    unsafe {
        println!("{}", (*raw).speak());
        let thin = raw as *mut u8 as *mut Dog;
        (*thin).0 = 9;
        println!("{}", (&*raw).speak());
        drop(Box::from_raw(raw));
    }
    let bs: Box<[u16]> = vec![1, 2, 3].into_boxed_slice();
    let rs: *mut [u16] = Box::into_raw(bs);
    unsafe {
        println!("len {}", (&*rs).len());
        (*rs)[2] = 30;
        let back = Box::from_raw(rs);
        println!("{:?}", back);
    }
    let bstr: Box<str> = "boxed str".into();
    let rstr = Box::into_raw(bstr);
    unsafe {
        (*rstr).make_ascii_uppercase();
        println!("{}", Box::from_raw(rstr));
    }

    let a = MyRc::new(String::from("shared"));
    let b2 = a.clone();
    {
        let c = b2.clone();
        println!("{} {} count {}", *a, c.len(), a.count());
    }
    println!("count after {}", a.count());
    drop(a);
    println!("still {} {}", *b2, b2.count());
    let d = MyRc::new(Dog(1));
    let d2 = d.clone();
    drop(d);
    println!("dog alive: {}", d2.speak());
    drop(d2);

    let mut mv = MiniVec::new();
    for i in 0..10 {
        mv.push(format!("s{}", i));
    }
    println!("{:?} {:?} cap {}", mv.pop(), mv.pop(), mv.cap);
    let mut zv = MiniVec::new();
    for _ in 0..1000 {
        zv.push(Unit);
    }
    let zl = zv.len;
    println!("zst len {} pop {:?}", zl, zv.pop());
    let mut dv = MiniVec::new();
    dv.push(Dog(50));
    dv.push(Dog(51));
    drop(dv);

    let words = [0x0100u32, 0x0200, 0x0300, 0x0400];
    let bytes: &[u8] = unsafe { std::slice::from_raw_parts(words.as_ptr() as *const u8, 15) };
    let (pre, mid, post) = unsafe { bytes[1..].align_to::<u32>() };
    println!("align_to lens {} {} {} sum {}", pre.len(), mid.len(), post.len(), pre.len() + mid.len() * 4 + post.len());
    let s = String::from("from raw parts");
    let mut s = mem::ManuallyDrop::new(s);
    let s2 = unsafe { String::from_raw_parts(s.as_mut_ptr(), s.len(), s.capacity()) };
    println!("{}", s2);

    let st = Arc::new(Stack { head: AtomicPtr::new(ptr::null_mut()) });
    let pushed = Arc::new(AtomicUsize::new(0));
    let hs: Vec<_> = (0..4)
        .map(|t| {
            let st = st.clone();
            let pushed = pushed.clone();
            thread::spawn(move || {
                for i in 0..250 {
                    st.push(t * 1000 + i);
                    pushed.fetch_add(1, Ordering::Relaxed);
                }
            })
        })
        .collect();
    for h in hs {
        h.join().unwrap();
    }
    let mut all = st.drain();
    all.sort();
    all.dedup();
    println!("treiber {} unique {} first {:?} last {:?}", pushed.load(Ordering::Relaxed), all.len(), all.first(), all.last());
}
