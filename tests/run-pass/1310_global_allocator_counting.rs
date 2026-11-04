// A #[global_allocator] wrapping System that counts allocations and bytes, used to
// observe Vec growth, Box, String and with_capacity behavior; Layout helpers.
// (Measurements are taken without printing in between: formatting may allocate.)
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

struct Counting;

static ALLOCS: AtomicUsize = AtomicUsize::new(0);
static FREES: AtomicUsize = AtomicUsize::new(0);
static LIVE: AtomicUsize = AtomicUsize::new(0);
static MAX_ALIGN: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::SeqCst);
        LIVE.fetch_add(layout.size(), Ordering::SeqCst);
        MAX_ALIGN.fetch_max(layout.align(), Ordering::SeqCst);
        System.alloc(layout)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        FREES.fetch_add(1, Ordering::SeqCst);
        LIVE.fetch_sub(layout.size(), Ordering::SeqCst);
        System.dealloc(ptr, layout)
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        LIVE.fetch_add(new_size, Ordering::SeqCst);
        LIVE.fetch_sub(layout.size(), Ordering::SeqCst);
        System.realloc(ptr, layout, new_size)
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

fn snapshot() -> (usize, usize, usize) {
    (ALLOCS.load(Ordering::SeqCst), FREES.load(Ordering::SeqCst), LIVE.load(Ordering::SeqCst))
}

#[repr(align(256))]
struct Page([u8; 256]);

fn main() {
    let mut out: Vec<String> = Vec::with_capacity(64);
    let (a0, f0, l0) = snapshot();
    let b = Box::new([0u64; 16]);
    let (a1, _, l1) = snapshot();
    drop(b);
    let (_, f2, l2) = snapshot();
    out.push(format!("box: allocs +{} live +{}; after drop frees +{} live delta {}", a1 - a0, l1 - l0, f2 - f0, l2 as isize - l0 as isize));

    let (a3, _, l3) = snapshot();
    let mut v: Vec<u32> = Vec::with_capacity(10);
    let (a4, _, l4) = snapshot();
    for i in 0..10 {
        v.push(i);
    }
    let (a5, _, _) = snapshot();
    v.push(10);
    let (a6, _, _) = snapshot();
    let grown = v.capacity() >= 11;
    v.shrink_to_fit();
    let shrunk = (v.capacity(), v.len());
    drop(v);
    out.push(format!("with_capacity(10): allocs +{} bytes +{}; pushes within: +{}; growth allocs +{} (realloc) grown {}", a4 - a3, l4 - l3, a5 - a4, a6 - a5, grown));
    out.push(format!("shrunk {:?}", shrunk));

    let (a7, _, _) = snapshot();
    let empty: Vec<u64> = Vec::new();
    let s = String::new();
    let e2: Vec<()> = vec![(); 1000];
    let (a8, _, _) = snapshot();
    out.push(format!("empty vec/string/zst vec allocs +{} (len {} {} {})", a8 - a7, empty.len(), s.len(), e2.len()));

    let p = Box::new(Page([7; 256]));
    out.push(format!("page aligned {} max_align>=256 {} byte {}", (&*p as *const Page as usize) % 256 == 0, MAX_ALIGN.load(Ordering::SeqCst) >= 256, p.0[255]));
    drop(p);

    let l = Layout::for_value(&[1u16, 2, 3]);
    out.push(format!("for_value {} {}", l.size(), l.align()));
    out.push(format!("array overflow err {}", Layout::array::<u64>(usize::MAX / 4).is_err()));
    let l2 = Layout::new::<u8>().align_to(16).unwrap();
    out.push(format!("align_to {} {}", l2.size(), l2.align()));
    out.push(format!("pad_to_align {}", Layout::from_size_align(13, 8).unwrap().pad_to_align().size()));

    let before = LIVE.load(Ordering::SeqCst);
    let mut total = 0;
    {
        let strings: Vec<String> = (0..50).map(|i| i.to_string() + "-item").collect();
        total += strings.iter().map(|s| s.len()).sum::<usize>();
    }
    let after = LIVE.load(Ordering::SeqCst);
    out.push(format!("strings total len {} no leak {}", total, after == before));
    let (a9, f9, _) = snapshot();
    out.push(format!("allocs>=frees {}", a9 >= f9));
    for line in &out {
        println!("{}", line);
    }
}
