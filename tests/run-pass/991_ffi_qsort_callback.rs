use std::os::raw::{c_int, c_void};
use std::cmp::Ordering;

extern "C" {
    fn qsort(base: *mut c_void, n: usize, size: usize, cmp: extern "C" fn(*const c_void, *const c_void) -> c_int);
    fn bsearch(key: *const c_void, base: *const c_void, n: usize, size: usize,
               cmp: extern "C" fn(*const c_void, *const c_void) -> c_int) -> *mut c_void;
}

extern "C" fn cmp_i32(a: *const c_void, b: *const c_void) -> c_int {
    let (a, b) = unsafe { (*(a as *const i32), *(b as *const i32)) };
    match a.cmp(&b) {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

extern "C" fn cmp_desc(a: *const c_void, b: *const c_void) -> c_int {
    -cmp_i32(a, b)
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct Rec {
    key: u16,
    score: f64,
}

extern "C" fn cmp_rec(a: *const c_void, b: *const c_void) -> c_int {
    let a = unsafe { &*(a as *const Rec) };
    let b = unsafe { &*(b as *const Rec) };
    b.score.partial_cmp(&a.score).unwrap() as c_int
}

fn sort_with(v: &mut [i32], f: extern "C" fn(*const c_void, *const c_void) -> c_int) {
    unsafe { qsort(v.as_mut_ptr() as *mut c_void, v.len(), std::mem::size_of::<i32>(), f); }
}

#[no_mangle]
pub extern "C" fn rust_exported(x: i32) -> i32 { x * 2 + 1 }

fn main() {
    let mut v = vec![5, -3, 99, 0, 42, 7, 7, -100];
    sort_with(&mut v, cmp_i32);
    println!("{:?}", v);
    let key = 42i32;
    let found = unsafe {
        bsearch(&key as *const i32 as *const c_void, v.as_ptr() as *const c_void, v.len(), 4, cmp_i32)
    };
    if !found.is_null() {
        let idx = (found as usize - v.as_ptr() as usize) / 4;
        println!("found 42 at {}", idx);
    }
    let missing = 1i32;
    let nf = unsafe { bsearch(&missing as *const i32 as *const c_void, v.as_ptr() as *const c_void, v.len(), 4, cmp_i32) };
    println!("missing null: {}", nf.is_null());
    sort_with(&mut v, cmp_desc);
    println!("{:?}", v);
    let mut recs = [
        Rec { key: 1, score: 2.5 },
        Rec { key: 2, score: 9.0 },
        Rec { key: 3, score: -1.0 },
        Rec { key: 4, score: 4.25 },
    ];
    unsafe {
        qsort(recs.as_mut_ptr() as *mut c_void, recs.len(), std::mem::size_of::<Rec>(), cmp_rec);
    }
    for r in &recs { println!("{} {}", r.key, r.score); }
    let fp: extern "C" fn(i32) -> i32 = rust_exported;
    println!("exported {}", fp(20));
    let fns: [(&str, extern "C" fn(*const c_void, *const c_void) -> c_int); 2] = [("asc", cmp_i32), ("desc", cmp_desc)];
    for (name, f) in fns.iter() {
        let mut w = [3, 1, 2];
        sort_with(&mut w, *f);
        println!("{} {:?}", name, w);
    }
}
