use std::collections::HashSet;
use std::os::raw::{c_int, c_void};

extern "C" fn add1(x: i32) -> i32 { x + 1 }
extern "C" fn dbl(x: i32) -> i32 { x * 2 }
unsafe fn raw_read(p: *const i32) -> i32 { *p }
unsafe extern "C" fn uc(x: i32) -> i32 { x - 1 }
fn plain(x: i32) -> i32 { x + 100 }
extern fn noabi(x: i32) -> i32 { x + 7 }

trait Describe { fn describe(&self) -> String; }
impl Describe for extern "C" fn(i32) -> i32 {
    fn describe(&self) -> String { format!("C callback -> {}", self(10)) }
}
impl Describe for fn(i32) -> i32 {
    fn describe(&self) -> String { format!("Rust fn -> {}", self(10)) }
}
impl Describe for unsafe fn(*const i32) -> i32 {
    fn describe(&self) -> String { let v = 5; format!("unsafe fn -> {}", unsafe { self(&v) }) }
}

extern "C" {
    fn qsort(base: *mut c_void, n: usize, sz: usize, cmp: Option<unsafe extern "C" fn(*const c_void, *const c_void) -> c_int>);
    fn abs(x: c_int) -> c_int;
}

unsafe extern "C" fn cmp_i32(a: *const c_void, b: *const c_void) -> c_int {
    let a = *(a as *const i32);
    let b = *(b as *const i32);
    a.cmp(&b) as c_int
}
extern "C" fn cmp_desc(a: *const c_void, b: *const c_void) -> c_int {
    unsafe { (*(b as *const i32)).cmp(&*(a as *const i32)) as c_int }
}

struct Handlers {
    on_event: Option<extern "C" fn(i32) -> i32>,
    raw: unsafe fn(*const i32) -> i32,
}

fn apply_all(fs: &[extern "C" fn(i32) -> i32], x: i32) -> Vec<i32> {
    fs.iter().map(|f| f(x)).collect()
}

fn takes_unsafe(f: unsafe fn(i32) -> i32, x: i32) -> i32 { unsafe { f(x) } }
fn takes_fn<F: Fn(i32) -> i32>(f: F) -> i32 { f(3) }

fn main() {
    let p: extern "C" fn(i32) -> i32 = add1;
    let q: extern "C" fn(i32) -> i32 = dbl;
    println!("{} {}", p(1), q(4));
    println!("{}", p == add1 as extern "C" fn(i32) -> i32);
    println!("{}", p == q);
    let _ = format!("{:?}", p);
    let mut set = HashSet::new();
    set.insert(p);
    set.insert(q);
    set.insert(p);
    println!("set {}", set.len());
    let c = p;
    let d = c.clone();
    println!("{}", d(5));
    println!("{}", p.describe());
    let r: fn(i32) -> i32 = plain;
    println!("{}", r.describe());
    let u: unsafe fn(*const i32) -> i32 = raw_read;
    println!("{}", u.describe());
    let u2: unsafe extern "C" fn(i32) -> i32 = uc;
    let u3: unsafe extern "C" fn(i32) -> i32 = add1;
    unsafe { println!("{} {}", u2(10), u3(10)); }
    let n: extern "C" fn(i32) -> i32 = noabi;
    println!("{}", n(0));
    // safe -> unsafe coercions
    println!("{}", takes_unsafe(plain, 1));
    println!("{}", takes_unsafe(r, 2));
    println!("{}", takes_unsafe(|x| x * 3, 3));
    let cl: unsafe fn(i32) -> i32 = |x| x + 1000;
    println!("{}", unsafe { cl(1) });
    // safe Rust-ABI fn items and pointers implement Fn
    println!("{} {}", takes_fn(plain), takes_fn(r));
    // Option niche
    println!("{}", std::mem::size_of::<Option<extern "C" fn()>>() == std::mem::size_of::<usize>());
    println!("{}", std::mem::size_of::<Option<unsafe extern "C" fn(i32, ...) -> i32>>());
    let h = Handlers { on_event: Some(add1), raw: raw_read };
    if let Some(f) = h.on_event { println!("event {}", f(41)); }
    let v = 9;
    println!("raw {}", unsafe { (h.raw)(&v) });
    println!("{:?}", apply_all(&[add1, dbl, noabi], 5));
    let mut arr = [5, 3, 9, 1, 7];
    unsafe {
        qsort(arr.as_mut_ptr() as *mut c_void, arr.len(), 4, Some(cmp_i32));
    }
    println!("{:?}", arr);
    unsafe {
        qsort(arr.as_mut_ptr() as *mut c_void, arr.len(), 4, Some(cmp_desc));
    }
    println!("{:?}", arr);
    let a: unsafe extern "C" fn(c_int) -> c_int = abs;
    println!("{}", unsafe { a(-4) });
    let addr = p as usize;
    println!("{}", addr != 0);
    let tn = std::any::type_name::<unsafe extern "C" fn(i32) -> i32>();
    println!("{}", tn);
    println!("{}", std::any::type_name::<extern "C" fn(i32, ...)>());
    println!("{}", std::any::type_name::<unsafe fn()>());
    let o: Option<fn(i32) -> i32> = Some(plain);
    println!("{:?}", o.map(|f| f(1)));
}
