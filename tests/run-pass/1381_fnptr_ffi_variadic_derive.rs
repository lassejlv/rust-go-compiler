use std::collections::BTreeMap;
use std::ffi::{c_char, c_int, CStr};

extern "C" {
    fn snprintf(buf: *mut c_char, n: usize, fmt: *const c_char, ...) -> c_int;
    fn strlen(s: *const c_char) -> usize;
}

type Formatter = unsafe extern "C" fn(*mut c_char, usize, *const c_char, ...) -> c_int;
type Callback = extern "C" fn(i32) -> i32;
type UnsafeCb = unsafe fn(&mut Vec<i32>, usize);

struct S;
impl S {
    extern "C" fn triple(x: i32) -> i32 { x * 3 }
    unsafe fn push_n(v: &mut Vec<i32>, n: usize) { for i in 0..n { v.push(i as i32); } }
    fn plain(x: i32) -> i32 { x - 3 }
}

trait Op {
    unsafe fn apply(x: i32) -> i32;
    fn safe_apply(x: i32) -> i32;
}
struct Neg;
impl Op for Neg {
    unsafe fn apply(x: i32) -> i32 { -x }
    fn safe_apply(x: i32) -> i32 { -x * 10 }
}

fn id<T>(x: T) -> T { x }

extern "system" fn sys(x: i32) -> i32 { x + 1 }

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
struct Table {
    cb: Callback,
    raw: Option<unsafe extern "C" fn() -> i32>,
}

extern "C" fn forty_two() -> i32 { 42 }
extern "C" fn inc(x: i32) -> i32 { x + 1 }

fn run_ops(ops: &[unsafe fn(i32) -> i32], x: i32) -> i32 {
    ops.iter().fold(x, |acc, f| unsafe { f(acc) })
}

fn pick(b: bool) -> unsafe fn(i32) -> i32 {
    if b { Neg::apply } else { id::<i32> }
}

fn main() {
    let f: Formatter = snprintf;
    let mut buf = [0 as c_char; 64];
    let n = unsafe { f(buf.as_mut_ptr(), 64, c"%d-%s-%d".as_ptr(), 7 as c_int, c"x".as_ptr(), 9 as c_int) };
    let s = unsafe { CStr::from_ptr(buf.as_ptr()) };
    println!("{} {:?} {}", n, s, unsafe { strlen(buf.as_ptr()) });

    let t: Callback = S::triple;
    println!("{}", t(5));
    let pn: UnsafeCb = S::push_n;
    let mut v = Vec::new();
    unsafe { pn(&mut v, 4) };
    println!("{:?}", v);
    let pl: unsafe fn(i32) -> i32 = S::plain;
    println!("{}", unsafe { pl(10) });

    let a: unsafe fn(i32) -> i32 = <Neg as Op>::apply;
    let b: fn(i32) -> i32 = Neg::safe_apply;
    let c: unsafe fn(i32) -> i32 = b;
    println!("{} {} {}", unsafe { a(4) }, b(4), unsafe { c(4) });
    let g: unsafe fn(i32) -> i32 = id;
    println!("{}", unsafe { g(77) });
    println!("{}", run_ops(&[a, c, g, |x| x + 1, id::<i32>], 3));
    println!("{} {}", unsafe { pick(true)(5) }, unsafe { pick(false)(5) });

    let sy: extern "system" fn(i32) -> i32 = sys;
    println!("{}", sy(1));

    let tb = Table { cb: inc, raw: Some(forty_two) };
    let tb2 = tb;
    println!("{}", tb == tb2);
    println!("{:?}", tb.cmp(&tb2));
    println!("{}", (tb.cb)(1));
    if let Some(r) = tb.raw { println!("{}", unsafe { r() }); }
    let none: Option<unsafe extern "C" fn() -> i32> = None;
    println!("{}", none.is_none());
    let dbg = format!("{:?}", tb);
    println!("{}", dbg.starts_with("Table { cb: 0x"));
    println!("{}", format!("{:p}", tb.cb).starts_with("0x"));

    let mut m: BTreeMap<extern "C" fn(i32) -> i32, &str> = BTreeMap::new();
    m.insert(inc, "inc");
    m.insert(S::triple, "triple");
    m.insert(inc, "inc2");
    println!("{}", m.len());
    println!("{}", m.values().any(|v| *v == "inc2"));

    let p1: fn(i32) -> i32 = S::plain;
    let p2 = p1 as unsafe fn(i32) -> i32;
    let p3 = S::plain as unsafe fn(i32) -> i32;
    println!("{} {}", unsafe { p2(1) }, unsafe { p3(2) });
    println!("{}", std::ptr::fn_addr_eq(p1, p1));
    let addr = forty_two as usize;
    let back: extern "C" fn() -> i32 = unsafe { std::mem::transmute(addr) };
    println!("{}", back());
    let raw = inc as *const ();
    let back2 = unsafe { std::mem::transmute::<*const (), unsafe extern "C" fn(i32) -> i32>(raw) };
    println!("{}", unsafe { back2(9) });
    let arr: [extern "C" fn(i32) -> i32; 2] = [inc, S::triple];
    println!("{:?}", arr.iter().map(|f| f(2)).collect::<Vec<_>>());
    let boxed: Box<dyn Fn(i32) -> i32> = Box::new(b);
    println!("{}", boxed(1));
    let fns: Vec<fn(i32) -> i32> = vec![b, S::plain, |x| x * x];
    println!("{:?}", fns.iter().map(|f| f(6)).collect::<Vec<_>>());
    println!("{:?}", fns.iter().copied().map(|f| f(2)).sum::<i32>());
    println!("{:?}", [1, 2, 3].map(S::plain));
    println!("{}", std::any::type_name::<Formatter>());
    println!("{}", std::any::type_name::<UnsafeCb>());
    println!("{}", std::any::type_name::<extern "system" fn(i32) -> i32>());
}
