use std::alloc::{alloc, dealloc, realloc, Layout};
use std::ptr;

struct MyVec<T> {
    ptr: *mut T,
    cap: usize,
    len: usize,
}

impl<T> MyVec<T> {
    fn new() -> Self { MyVec { ptr: ptr::null_mut(), cap: 0, len: 0 } }
    fn grow(&mut self) {
        let new_cap = if self.cap == 0 { 2 } else { self.cap * 2 };
        let new_layout = Layout::array::<T>(new_cap).unwrap();
        let new_ptr = if self.cap == 0 {
            unsafe { alloc(new_layout) }
        } else {
            let old = Layout::array::<T>(self.cap).unwrap();
            unsafe { realloc(self.ptr as *mut u8, old, new_layout.size()) }
        };
        assert!(!new_ptr.is_null());
        println!("grow {} -> {}", self.cap, new_cap);
        self.ptr = new_ptr as *mut T;
        self.cap = new_cap;
    }
    fn push(&mut self, v: T) {
        if self.len == self.cap { self.grow(); }
        unsafe { ptr::write(self.ptr.add(self.len), v); }
        self.len += 1;
    }
    fn pop(&mut self) -> Option<T> {
        if self.len == 0 { return None; }
        self.len -= 1;
        Some(unsafe { ptr::read(self.ptr.add(self.len)) })
    }
    fn insert(&mut self, idx: usize, v: T) {
        assert!(idx <= self.len);
        if self.len == self.cap { self.grow(); }
        unsafe {
            ptr::copy(self.ptr.add(idx), self.ptr.add(idx + 1), self.len - idx);
            ptr::write(self.ptr.add(idx), v);
        }
        self.len += 1;
    }
    fn remove(&mut self, idx: usize) -> T {
        assert!(idx < self.len);
        unsafe {
            self.len -= 1;
            let r = ptr::read(self.ptr.add(idx));
            ptr::copy(self.ptr.add(idx + 1), self.ptr.add(idx), self.len - idx);
            r
        }
    }
    fn as_slice(&self) -> &[T] {
        if self.len == 0 { return &[]; }
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }
}

impl<T> Drop for MyVec<T> {
    fn drop(&mut self) {
        while let Some(_) = self.pop() {}
        if self.cap > 0 {
            unsafe { dealloc(self.ptr as *mut u8, Layout::array::<T>(self.cap).unwrap()); }
            println!("dealloc cap {}", self.cap);
        }
    }
}

struct Loud(String);
impl Drop for Loud {
    fn drop(&mut self) { println!("drop {}", self.0); }
}

fn main() {
    let mut v = MyVec::new();
    for i in 0..5 { v.push(i * i); }
    v.insert(1, 100);
    let r = v.remove(3);
    println!("removed {} -> {:?}", r, v.as_slice());
    println!("pop {:?}", v.pop());
    let l = Layout::new::<u64>();
    println!("layout u64 size={} align={}", l.size(), l.align());
    let l2 = Layout::from_size_align(24, 8).unwrap();
    println!("l2 {} {}", l2.size(), l2.align());
    let arr = Layout::array::<u16>(7).unwrap();
    println!("arr {} {}", arr.size(), arr.align());
    let (ext, off) = Layout::new::<u8>().extend(Layout::new::<u32>()).unwrap();
    println!("extend size={} off={} padded={}", ext.size(), off, ext.pad_to_align().size());
    println!("bad align: {}", Layout::from_size_align(8, 3).is_err());
    {
        let mut s = MyVec::new();
        s.push(Loud("a".into()));
        s.push(Loud("b".into()));
        s.push(Loud("c".into()));
        let b = s.remove(1);
        println!("got {}", b.0);
    }
    unsafe {
        let lay = Layout::array::<u32>(4).unwrap();
        let p = std::alloc::alloc_zeroed(lay) as *mut u32;
        *p.add(2) = 9;
        let s = std::slice::from_raw_parts(p, 4);
        println!("zeroed {:?}", s);
        dealloc(p as *mut u8, lay);
    }
}
