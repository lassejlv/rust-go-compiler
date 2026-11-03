use std::mem::{self, ManuallyDrop};

struct Tracker(&'static str);
impl Drop for Tracker {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Holder {
    a: ManuallyDrop<Tracker>,
    b: Tracker,
}
impl Drop for Holder {
    fn drop(&mut self) {
        println!("Holder::drop start");
        unsafe { ManuallyDrop::drop(&mut self.a); }
        println!("Holder::drop end");
    }
}

fn main() {
    let b = Box::new(Tracker("boxed"));
    let raw = Box::into_raw(b);
    unsafe {
        println!("raw holds {}", (*raw).0);
        let b2 = Box::from_raw(raw);
        println!("reboxed {}", b2.0);
    }
    println!("--");
    let leaked: &'static mut Vec<i32> = Box::leak(Box::new(vec![1, 2, 3]));
    leaked.push(4);
    println!("leaked {:?}", leaked);
    let t = Tracker("forgotten");
    mem::forget(t);
    println!("after forget");
    let md = ManuallyDrop::new(Tracker("md never dropped"));
    println!("md {}", md.0);
    let mut md2 = ManuallyDrop::new(Tracker("md2"));
    let inner = unsafe { ManuallyDrop::take(&mut md2) };
    println!("took {}", inner.0);
    drop(inner);
    let md3 = ManuallyDrop::new(Tracker("md3"));
    let x = ManuallyDrop::into_inner(md3);
    println!("into_inner {}", x.0);
    {
        let _h = Holder { a: ManuallyDrop::new(Tracker("h.a")), b: Tracker("h.b") };
    }
    println!("--");
    let mut v = vec![Tracker("v0"), Tracker("v1")];
    let old = mem::replace(&mut v[0], Tracker("v0new"));
    println!("replaced out {}", old.0);
    let taken = mem::take(&mut String::from("tk"));
    println!("taken {}", taken);
    let mut a = Tracker("swapA");
    let mut c = Tracker("swapC");
    mem::swap(&mut a, &mut c);
    println!("a={} c={}", a.0, c.0);
    let p: *mut Tracker = Box::into_raw(Box::new(Tracker("drop_in_place")));
    unsafe {
        std::ptr::drop_in_place(p);
        std::alloc::dealloc(p as *mut u8, std::alloc::Layout::new::<Tracker>());
    }
    println!("needs_drop: {} {}", mem::needs_drop::<Tracker>(), mem::needs_drop::<u32>());
    println!("end of main");
}
