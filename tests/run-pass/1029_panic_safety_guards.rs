use std::panic::{self, AssertUnwindSafe};
use std::ptr;

struct Noisy(i32);
impl Drop for Noisy {
    fn drop(&mut self) { println!("drop Noisy({})", self.0); }
}

// Fill a buffer of uninitialized slots; on panic, drop only the initialized prefix.
fn fill_with<F: FnMut(usize) -> Noisy>(n: usize, mut f: F) -> Vec<Noisy> {
    struct Guard { ptr: *mut Noisy, init: usize }
    impl Drop for Guard {
        fn drop(&mut self) {
            println!("guard cleaning {} items", self.init);
            unsafe { ptr::drop_in_place(std::slice::from_raw_parts_mut(self.ptr, self.init)); }
        }
    }
    let mut v: Vec<Noisy> = Vec::with_capacity(n);
    let mut g = Guard { ptr: v.as_mut_ptr(), init: 0 };
    for i in 0..n {
        unsafe { g.ptr.add(i).write(f(i)); }
        g.init += 1;
    }
    std::mem::forget(g);
    unsafe { v.set_len(n); }
    v
}

struct Defer<F: FnMut()>(F);
impl<F: FnMut()> Drop for Defer<F> {
    fn drop(&mut self) { (self.0)(); }
}

fn main() {
    panic::set_hook(Box::new(|info| {
        let msg = info.payload().downcast_ref::<&str>().copied().unwrap_or("?");
        println!("hook: {}", msg);
    }));
    let v = fill_with(3, |i| Noisy(i as i32));
    println!("ok len {}", v.len());
    drop(v);
    let r = panic::catch_unwind(|| {
        fill_with(5, |i| if i == 3 { panic!("fail at 3") } else { Noisy(10 + i as i32) })
    });
    println!("caught {}", r.is_err());
    let mut counter = 0;
    let r = panic::catch_unwind(AssertUnwindSafe(|| {
        let _d = Defer(|| println!("deferred runs during unwind"));
        counter += 1;
        let _n = Noisy(99);
        if counter > 0 { panic!("unwinding"); }
        5
    }));
    println!("result err {} counter {}", r.is_err(), counter);
    let r = panic::catch_unwind(|| {
        let v = vec![Noisy(1), Noisy(2)];
        let _it = v.into_iter().map(|n| if n.0 == 2 { panic!("in map") } else { n }).collect::<Vec<_>>();
    });
    println!("map panic caught {}", r.is_err());
    let r: Result<i32, _> = panic::catch_unwind(|| 42);
    println!("{:?}", r.ok());
    let r = panic::catch_unwind(|| { panic::resume_unwind(Box::new(7u8)); });
    println!("resume payload {:?}", r.unwrap_err().downcast_ref::<u8>());
    let _ = panic::take_hook();
    println!("end");
}
