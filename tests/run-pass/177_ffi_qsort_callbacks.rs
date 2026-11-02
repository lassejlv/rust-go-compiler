use std::ffi::{c_int, c_void};

extern "C" {
    fn qsort(base: *mut c_void, n: usize, size: usize, cmp: extern "C" fn(*const c_void, *const c_void) -> c_int);
    fn bsearch(
        key: *const c_void,
        base: *const c_void,
        n: usize,
        size: usize,
        cmp: extern "C" fn(*const c_void, *const c_void) -> c_int,
    ) -> *mut c_void;
}

extern "C" fn cmp_i32(a: *const c_void, b: *const c_void) -> c_int {
    let (a, b) = unsafe { (*(a as *const i32), *(b as *const i32)) };
    if a < b {
        -1
    } else if a > b {
        1
    } else {
        0
    }
}

extern "C" fn cmp_i32_desc(a: *const c_void, b: *const c_void) -> c_int {
    -cmp_i32(a, b)
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct Item {
    weight: u32,
    id: u16,
}

extern "C" fn cmp_item(a: *const c_void, b: *const c_void) -> c_int {
    let a = unsafe { &*(a as *const Item) };
    let b = unsafe { &*(b as *const Item) };
    match a.weight.cmp(&b.weight).then(a.id.cmp(&b.id)) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

extern "C" fn cmp_f64(a: *const c_void, b: *const c_void) -> c_int {
    let (a, b) = unsafe { (*(a as *const f64), *(b as *const f64)) };
    a.partial_cmp(&b).map(|o| o as c_int).unwrap_or(0)
}

fn sort_with<T>(v: &mut [T], f: extern "C" fn(*const c_void, *const c_void) -> c_int) {
    unsafe { qsort(v.as_mut_ptr() as *mut c_void, v.len(), std::mem::size_of::<T>(), f) }
}

extern "C" fn twice(x: i32) -> i32 {
    x * 2
}

fn call_c_fn(f: extern "C" fn(i32) -> i32, v: i32) -> i32 {
    f(v)
}

fn main() {
    let mut v = vec![42, -7, 19, 0, 3, 3, 100, -50];
    sort_with(&mut v, cmp_i32);
    println!("{:?}", v);
    for key in [19, 4] {
        let p = unsafe {
            bsearch(&key as *const i32 as *const c_void, v.as_ptr() as *const c_void, v.len(), 4, cmp_i32)
        };
        if p.is_null() {
            println!("{} not found", key);
        } else {
            let idx = (p as usize - v.as_ptr() as usize) / 4;
            println!("{} found at index {}", key, idx);
        }
    }
    sort_with(&mut v, cmp_i32_desc);
    println!("{:?}", v);

    let mut items = [
        Item { weight: 5, id: 3 },
        Item { weight: 1, id: 9 },
        Item { weight: 5, id: 1 },
        Item { weight: 2, id: 2 },
    ];
    sort_with(&mut items, cmp_item);
    for it in &items {
        println!("{:?}", it);
    }
    let mut fs = [2.5f64, -1.25, 9.0, 0.0];
    sort_with(&mut fs, cmp_f64);
    println!("{:?}", fs);

    let fp: extern "C" fn(i32) -> i32 = twice;
    println!("{}", call_c_fn(fp, 21));
    let table: [(&str, extern "C" fn(*const c_void, *const c_void) -> c_int); 2] = [("asc", cmp_i32), ("desc", cmp_i32_desc)];
    for (name, f) in table {
        let mut w = [3, 1, 2];
        sort_with(&mut w, f);
        println!("{} {:?}", name, w);
    }
}
