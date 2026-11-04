// FFI callbacks through libc (qsort, bsearch) with `extern "C"` and
// `unsafe extern "C"` fn pointers, a C-style vtable of callbacks over a
// raw context pointer, and raw-pointer code that takes its references
// explicitly (no implicit autorefs of raw pointer derefs).
use std::ffi::{c_int, c_void};
use std::ptr;

type Compare = unsafe extern "C" fn(*const c_void, *const c_void) -> c_int;

extern "C" {
    fn qsort(base: *mut c_void, n: usize, size: usize, cmp: Option<Compare>);
    fn bsearch(key: *const c_void, base: *const c_void, n: usize, size: usize, cmp: Option<Compare>) -> *mut c_void;
}

unsafe extern "C" fn cmp_u64(a: *const c_void, b: *const c_void) -> c_int {
    let (a, b) = (*(a as *const u64), *(b as *const u64));
    if a < b { -1 } else if a > b { 1 } else { 0 }
}

// a safe extern "C" fn also coerces to the unsafe pointer type
extern "C" fn cmp_len(a: *const c_void, b: *const c_void) -> c_int {
    let (a, b) = unsafe { (&*(a as *const &str), &*(b as *const &str)) };
    a.len().cmp(&b.len()) as c_int
}

fn sort_with<T>(v: &mut [T], cmp: Compare) {
    unsafe { qsort(v.as_mut_ptr() as *mut c_void, v.len(), std::mem::size_of::<T>(), Some(cmp)) }
}

#[repr(C)]
struct Ops {
    push: unsafe extern "C" fn(*mut c_void, i32),
    total: extern "C" fn(*const c_void) -> i64,
    reset: Option<unsafe extern "C" fn(*mut c_void)>,
}

unsafe extern "C" fn vec_push(ctx: *mut c_void, x: i32) {
    let v = &mut *(ctx as *mut Vec<i32>);
    v.push(x);
}
extern "C" fn vec_total(ctx: *const c_void) -> i64 {
    let v = unsafe { &*(ctx as *const Vec<i32>) };
    v.iter().map(|&x| x as i64).sum()
}
unsafe extern "C" fn vec_reset(ctx: *mut c_void) {
    (&mut *(ctx as *mut Vec<i32>)).clear();
}

fn drive(ops: &Ops, ctx: *mut c_void) -> i64 {
    for i in 1..=5 {
        unsafe { (ops.push)(ctx, i * i) };
    }
    let t = (ops.total)(ctx);
    if let Some(r) = ops.reset {
        unsafe { r(ctx) };
    }
    t
}

struct Grid {
    cells: Vec<u8>,
    w: usize,
}

fn main() {
    let mut nums: Vec<u64> = vec![42, 7, 19, 3, 88, 21];
    sort_with(&mut nums, cmp_u64);
    println!("{:?}", nums);
    let key: u64 = 19;
    let hit = unsafe {
        bsearch(&key as *const u64 as *const c_void, nums.as_ptr() as *const c_void, nums.len(), 8, Some(cmp_u64))
    };
    println!("found at {}", (hit as usize - nums.as_ptr() as usize) / 8);

    let mut words = vec!["pear", "fig", "banana", "kiwi"];
    sort_with(&mut words, cmp_len);
    println!("{:?}", words);

    let ops = Ops { push: vec_push, total: vec_total, reset: Some(vec_reset) };
    let mut store: Vec<i32> = Vec::new();
    let ctx = &mut store as *mut Vec<i32> as *mut c_void;
    println!("total {}", drive(&ops, ctx));
    println!("after reset {:?}", store);
    let ops2 = Ops { reset: None, ..ops };
    println!("total {}", drive(&ops2, ctx));
    println!("kept {:?}", store);

    // raw pointers with explicit references
    let mut v = vec![10, 20, 30];
    let pv: *mut Vec<i32> = &mut v;
    let mut arr = [1u8, 2, 3, 4];
    let ps: *mut [u8] = &mut arr[..];
    let mut g = Grid { cells: vec![0; 6], w: 3 };
    let pg: *mut Grid = &mut g;
    unsafe {
        (&mut *pv)[1] += 1;
        let first = (&(*pv))[0];
        let n = (&*ps).len();
        let tail = &(&*ps)[2..];
        (*ps)[0] = 9; // builtin slice index: no reference is created
        let len_raw = ps.len(); // the raw slice pointer's own len
        let w = (*pg).w; // plain field read
        (&mut (*pg).cells)[w + 1] = 5;
        let cells = &(*pg).cells;
        let addr = ptr::addr_of!((*pg).w);
        println!("{} {} {:?} {} {} {:?} {}", first, n, tail, len_raw, w, cells, *addr);
        let pv2 = &mut *pv;
        pv2.push(40);
    }
    println!("{:?} {:?}", v, arr);
}
