// Vec::from_raw_parts, slice::from_raw_parts(_mut), std::alloc with Layout, a tiny bump arena.
use std::alloc::{alloc, alloc_zeroed, dealloc, realloc, Layout};
use std::mem::{self, ManuallyDrop};
use std::ptr;
use std::slice;

struct RawBuf<T> {
    ptr: *mut T,
    cap: usize,
    len: usize,
}

impl<T: std::fmt::Debug> RawBuf<T> {
    fn new() -> Self {
        RawBuf { ptr: ptr::null_mut(), cap: 0, len: 0 }
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
        assert!(!p.is_null());
        self.ptr = p as *mut T;
        self.cap = new_cap;
        println!("grew to {}", new_cap);
    }
    fn push(&mut self, v: T) {
        if self.len == self.cap {
            self.grow();
        }
        unsafe { self.ptr.add(self.len).write(v) };
        self.len += 1;
    }
    fn as_slice(&self) -> &[T] {
        if self.ptr.is_null() { &[] } else { unsafe { slice::from_raw_parts(self.ptr, self.len) } }
    }
}
impl<T> Drop for RawBuf<T> {
    fn drop(&mut self) {
        if self.cap > 0 {
            unsafe {
                ptr::drop_in_place(ptr::slice_from_raw_parts_mut(self.ptr, self.len));
                dealloc(self.ptr as *mut u8, Layout::array::<T>(self.cap).unwrap());
            }
        }
    }
}

struct Loud(u8);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop Loud({})", self.0);
    }
}
impl std::fmt::Debug for Loud {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "L{}", self.0)
    }
}

fn main() {
    let v = vec![1u32, 2, 3, 4];
    let mut v = ManuallyDrop::new(v);
    let (p, len, cap) = (v.as_mut_ptr(), v.len(), v.capacity());
    unsafe {
        for i in 0..len {
            *p.add(i) *= 10;
        }
        let rebuilt = Vec::from_raw_parts(p, len, cap);
        println!("{:?}", rebuilt);
    }

    let mut data = [5i64, 6, 7, 8, 9];
    let s = unsafe { slice::from_raw_parts_mut(data.as_mut_ptr().add(1), 3) };
    s.reverse();
    println!("{:?}", data);

    // alloc + Layout
    let layout = Layout::new::<[u32; 8]>();
    println!("layout size {} align {}", layout.size(), layout.align());
    let l2 = Layout::from_size_align(24, 8).unwrap();
    let (ext, off) = l2.extend(Layout::new::<u16>()).unwrap();
    println!("extend size {} off {} padded {}", ext.size(), off, ext.pad_to_align().size());
    println!("bad align {:?}", Layout::from_size_align(8, 3).is_err());
    unsafe {
        let p = alloc_zeroed(layout) as *mut u32;
        let all_zero = slice::from_raw_parts(p, 8).iter().all(|&x| x == 0);
        for i in 0..8 {
            *p.add(i) = (i as u32) << i;
        }
        println!("zeroed {} {:?}", all_zero, slice::from_raw_parts(p, 8));
        dealloc(p as *mut u8, layout);
    }

    let mut rb = RawBuf::new();
    for i in 0..5 {
        rb.push(Loud(i));
    }
    println!("{:?}", rb.as_slice());
    drop(rb);

    let mut strings = vec![String::from("a"), String::from("b")];
    strings.reserve(10);
    let mut md = ManuallyDrop::new(strings);
    let rebuilt: Vec<String> = unsafe { Vec::from_raw_parts(md.as_mut_ptr(), md.len(), md.capacity()) };
    println!("{:?} cap>=12 {}", rebuilt, rebuilt.capacity() >= 12);

    let mut l = Loud(100);
    let l2 = mem::replace(&mut l, Loud(101));
    mem::forget(l2);
    println!("forgot 100");
    let taken = mem::take(&mut vec![1, 2]);
    println!("{:?}", taken);
    let mut x = 1;
    let mut y = 2;
    mem::swap(&mut x, &mut y);
    println!("{} {}", x, y);
    let s: &[u8] = unsafe { slice::from_raw_parts(ptr::NonNull::dangling().as_ptr(), 0) };
    println!("empty {:?}", s);
    let bytes: &[u8] = unsafe { slice::from_raw_parts(&0x0102_0304u32 as *const u32 as *const u8, 4) };
    println!("le bytes {:?}", bytes);
}
