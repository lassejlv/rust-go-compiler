fn get() -> usize {
    mod inner {
        use core::sync::atomic::{AtomicUsize, Ordering::Relaxed};
        pub fn f() -> usize { let a = AtomicUsize::new(3); a.load(Relaxed) }
    }
    inner::f()
}
fn main() { println!("{}", get()); }
