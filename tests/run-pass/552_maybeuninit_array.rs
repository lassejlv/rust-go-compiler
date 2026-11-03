// MaybeUninit arrays, assume_init, uninit buffers, write via as_mut_ptr, ManuallyDrop, mem::forget
use std::mem::{self, ManuallyDrop, MaybeUninit};

struct Tracked(u32);
impl Drop for Tracked {
    fn drop(&mut self) {
        println!("drop Tracked({})", self.0);
    }
}

fn make_array<const N: usize>(f: impl Fn(usize) -> String) -> [String; N] {
    let mut arr: [MaybeUninit<String>; N] = [const { MaybeUninit::uninit() }; N];
    for (i, slot) in arr.iter_mut().enumerate() {
        slot.write(f(i));
    }
    unsafe { mem::transmute_copy::<_, [String; N]>(&ManuallyDrop::new(arr)) }
}

struct StackVec<T, const N: usize> {
    data: [MaybeUninit<T>; N],
    len: usize,
}
impl<T, const N: usize> StackVec<T, N> {
    fn new() -> Self {
        StackVec { data: [const { MaybeUninit::uninit() }; N], len: 0 }
    }
    fn push(&mut self, v: T) -> Result<(), T> {
        if self.len == N {
            return Err(v);
        }
        self.data[self.len].write(v);
        self.len += 1;
        Ok(())
    }
    fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        Some(unsafe { self.data[self.len].assume_init_read() })
    }
    fn as_slice(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.data.as_ptr() as *const T, self.len) }
    }
}
impl<T, const N: usize> Drop for StackVec<T, N> {
    fn drop(&mut self) {
        for i in 0..self.len {
            unsafe { self.data[i].assume_init_drop() };
        }
    }
}

fn main() {
    let arr: [String; 4] = make_array(|i| format!("item{}", i * i));
    println!("{:?}", arr);
    let mut x = MaybeUninit::<u64>::uninit();
    x.write(0xfeed);
    let v = unsafe { x.assume_init() };
    println!("{:#x}", v);
    let mut buf: [MaybeUninit<u8>; 8] = [MaybeUninit::uninit(); 8];
    for (i, b) in buf.iter_mut().enumerate() {
        *b = MaybeUninit::new(b'a' + i as u8);
    }
    let bytes: [u8; 8] = unsafe { mem::transmute(buf) };
    println!("{}", std::str::from_utf8(&bytes).unwrap());
    let mut p = MaybeUninit::<(i32, f64)>::uninit();
    unsafe {
        let ptr = p.as_mut_ptr();
        std::ptr::addr_of_mut!((*ptr).0).write(7);
        std::ptr::addr_of_mut!((*ptr).1).write(2.5);
    }
    let pair = unsafe { p.assume_init() };
    println!("{:?}", pair);
    let z: [i32; 4] = unsafe { MaybeUninit::zeroed().assume_init() };
    println!("{:?}", z);
    {
        let mut sv: StackVec<Tracked, 3> = StackVec::new();
        for i in 1..=4 {
            match sv.push(Tracked(i)) {
                Ok(()) => println!("pushed {}", i),
                Err(t) => println!("full, rejected {}", t.0),
            }
        }
        println!("len {} first {}", sv.as_slice().len(), sv.as_slice()[0].0);
        let popped = sv.pop();
        println!("popped {:?}", popped.as_ref().map(|t| t.0));
        drop(popped);
        println!("leaving scope");
    }
    let md = ManuallyDrop::new(Tracked(100));
    println!("manually drop holds {}", md.0);
    let mut md2 = ManuallyDrop::new(Tracked(200));
    unsafe { ManuallyDrop::drop(&mut md2) };
    let inner = ManuallyDrop::into_inner(ManuallyDrop::new(Tracked(300)));
    println!("into_inner {}", inner.0);
    mem::forget(Tracked(400));
    let s = String::from("leak me");
    let leaked: &'static mut str = s.leak();
    leaked.make_ascii_uppercase();
    println!("{}", leaked);
    let b = Box::new(Tracked(500));
    let r: &'static Tracked = Box::leak(b);
    println!("leaked box {}", r.0);
    let uninit_vec: Vec<u32> = {
        let mut v = Vec::with_capacity(5);
        let spare = v.spare_capacity_mut();
        for (i, s) in spare.iter_mut().take(5).enumerate() {
            s.write(i as u32 * 3);
        }
        unsafe { v.set_len(5) };
        v
    };
    println!("{:?}", uninit_vec);
    println!("{} {}", mem::size_of::<MaybeUninit<u64>>(), mem::align_of::<MaybeUninit<u16>>());
    println!("{}", mem::size_of::<ManuallyDrop<String>>() == mem::size_of::<String>());
    println!("end of main");
}
