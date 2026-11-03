use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

struct Counting;

static ALLOCS: AtomicUsize = AtomicUsize::new(0);
static DEALLOCS: AtomicUsize = AtomicUsize::new(0);
static BYTES: AtomicUsize = AtomicUsize::new(0);
static TRACK: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if TRACK.load(Ordering::Relaxed) {
            ALLOCS.fetch_add(1, Ordering::Relaxed);
            BYTES.fetch_add(layout.size(), Ordering::Relaxed);
        }
        System.alloc(layout)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if TRACK.load(Ordering::Relaxed) {
            DEALLOCS.fetch_add(1, Ordering::Relaxed);
        }
        System.dealloc(ptr, layout)
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

fn measure<R>(label: &str, f: impl FnOnce() -> R) -> R {
    ALLOCS.store(0, Ordering::SeqCst);
    DEALLOCS.store(0, Ordering::SeqCst);
    BYTES.store(0, Ordering::SeqCst);
    TRACK.store(true, Ordering::SeqCst);
    let r = f();
    TRACK.store(false, Ordering::SeqCst);
    let (a, d, b) = (ALLOCS.load(Ordering::SeqCst), DEALLOCS.load(Ordering::SeqCst), BYTES.load(Ordering::SeqCst));
    println!("{}: allocs={} deallocs={} bytes={}", label, a, d, b);
    r
}

fn main() {
    measure("nothing", || 1 + 1);
    measure("box u64", || { let b = Box::new(5u64); *b });
    measure("vec with_capacity 10 u32", || { let v: Vec<u32> = Vec::with_capacity(10); v.capacity() });
    let v = measure("vec reserve_exact", || { let mut v: Vec<u8> = Vec::new(); v.reserve_exact(100); v });
    println!("cap {}", v.capacity());
    measure("drop vec", move || drop(v));
    measure("empty vec/string", || { let v: Vec<i32> = Vec::new(); let s = String::new(); (v.len(), s.len()) });
    measure("string from", || String::from("hello world").len());
    measure("box zst", || { let b = Box::new(()); *b });
    measure("array on stack", || { let a = [0u8; 64]; a.len() });
}
