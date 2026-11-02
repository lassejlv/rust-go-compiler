use std::alloc::{alloc, dealloc, realloc, Layout};
use std::ptr;

// A minimal growable vector built on the raw allocator.
struct RawVec<T> {
    ptr: *mut T,
    cap: usize,
    len: usize,
}

impl<T> RawVec<T> {
    fn new() -> Self {
        RawVec { ptr: ptr::null_mut(), cap: 0, len: 0 }
    }
    fn grow(&mut self) {
        let new_cap = if self.cap == 0 { 2 } else { self.cap * 2 };
        let new_layout = Layout::array::<T>(new_cap).unwrap();
        let p = unsafe {
            if self.cap == 0 {
                alloc(new_layout)
            } else {
                realloc(self.ptr as *mut u8, Layout::array::<T>(self.cap).unwrap(), new_layout.size())
            }
        };
        self.ptr = p as *mut T;
        println!("  grow {} -> {}", self.cap, new_cap);
        self.cap = new_cap;
    }
    fn push(&mut self, v: T) {
        if self.len == self.cap {
            self.grow();
        }
        unsafe { ptr::write(self.ptr.add(self.len), v) };
        self.len += 1;
    }
    fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;
            Some(unsafe { ptr::read(self.ptr.add(self.len)) })
        }
    }
    fn as_slice(&self) -> &[T] {
        if self.len == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
        }
    }
    fn insert(&mut self, idx: usize, v: T) {
        assert!(idx <= self.len);
        if self.len == self.cap {
            self.grow();
        }
        unsafe {
            ptr::copy(self.ptr.add(idx), self.ptr.add(idx + 1), self.len - idx);
            ptr::write(self.ptr.add(idx), v);
        }
        self.len += 1;
    }
    fn remove(&mut self, idx: usize) -> T {
        assert!(idx < self.len);
        unsafe {
            let v = ptr::read(self.ptr.add(idx));
            ptr::copy(self.ptr.add(idx + 1), self.ptr.add(idx), self.len - idx - 1);
            self.len -= 1;
            v
        }
    }
}

impl<T> Drop for RawVec<T> {
    fn drop(&mut self) {
        while self.pop().is_some() {}
        if self.cap > 0 {
            unsafe { dealloc(self.ptr as *mut u8, Layout::array::<T>(self.cap).unwrap()) };
        }
        println!("  RawVec freed (cap {})", self.cap);
    }
}

#[derive(Debug)]
struct Loud(String);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("  drop Loud({})", self.0);
    }
}

fn main() {
    let mut v: RawVec<i64> = RawVec::new();
    for i in 0..9 {
        v.push(i * i);
    }
    println!("{:?}", v.as_slice());
    v.insert(0, -1);
    v.insert(5, 555);
    println!("{:?}", v.as_slice());
    println!("removed {}", v.remove(2));
    println!("pop {:?}", v.pop());
    println!("{:?} len {} cap {}", v.as_slice(), v.len, v.cap);
    drop(v);

    let mut s: RawVec<Loud> = RawVec::new();
    s.push(Loud("a".into()));
    s.push(Loud("b".into()));
    s.push(Loud("c".into()));
    let b = s.remove(1);
    println!("removed {:?}", b);
    drop(b);
    println!("remaining {:?}", s.as_slice());
    drop(s);

    let layout = Layout::new::<[u32; 4]>();
    println!("layout size {} align {}", layout.size(), layout.align());
    let l2 = Layout::from_size_align(24, 8).unwrap();
    println!("{} {}", l2.size(), l2.align());
    println!("{}", Layout::from_size_align(8, 3).is_err());
}
