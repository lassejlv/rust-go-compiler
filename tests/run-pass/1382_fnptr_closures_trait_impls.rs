// Closures and fn items coercing to safe and unsafe fn pointers; fn
// pointers of every safety/ABI combination as values, fields, generic
// arguments and trait implementors.
use std::collections::HashMap;
use std::fmt::Debug;

trait Callable {
    fn kind(&self) -> &'static str;
    fn invoke(&self, x: i64) -> i64;
}

impl Callable for fn(i64) -> i64 {
    fn kind(&self) -> &'static str { "fn" }
    fn invoke(&self, x: i64) -> i64 { self(x) }
}
impl Callable for unsafe fn(i64) -> i64 {
    fn kind(&self) -> &'static str { "unsafe fn" }
    fn invoke(&self, x: i64) -> i64 { unsafe { self(x) } }
}
impl Callable for extern "C" fn(i64) -> i64 {
    fn kind(&self) -> &'static str { "extern \"C\" fn" }
    fn invoke(&self, x: i64) -> i64 { self(x) }
}
impl Callable for unsafe extern "C" fn(i64) -> i64 {
    fn kind(&self) -> &'static str { "unsafe extern \"C\" fn" }
    fn invoke(&self, x: i64) -> i64 { unsafe { self(x) } }
}

// generic impls over fn pointer argument types
trait Arity { const N: usize; }
impl<R> Arity for fn() -> R { const N: usize = 0; }
impl<A, R> Arity for fn(A) -> R { const N: usize = 1; }
impl<A, B, R> Arity for fn(A, B) -> R { const N: usize = 2; }
impl<A, R> Arity for extern "C" fn(A) -> R { const N: usize = 101; }

fn arity<F: Arity>(_: F) -> usize { F::N }

fn sq(x: i64) -> i64 { x * x }
unsafe fn unchecked_half(x: i64) -> i64 { x / 2 }
extern "C" fn c_neg(x: i64) -> i64 { -x }
unsafe extern "C" fn c_twice(x: i64) -> i64 { 2 * x }

fn compose(f: fn(i64) -> i64, g: fn(i64) -> i64) -> impl Fn(i64) -> i64 {
    move |x| g(f(x))
}

fn apply_unsafe(fs: &[unsafe fn(i64) -> i64], x: i64) -> Vec<i64> {
    fs.iter().map(|f| unsafe { f(x) }).collect()
}

fn choose(op: char) -> fn(i64, i64) -> i64 {
    match op {
        '+' => |a, b| a + b,
        '-' => |a, b| a - b,
        '*' => |a, b| a * b,
        _ => |_, _| 0,
    }
}

fn choose_unsafe(op: u8) -> unsafe fn(i64) -> i64 {
    match op {
        0 => unchecked_half,
        1 => sq,
        _ => |x| x + 1,
    }
}

fn show<T: Debug>(t: T) -> String { format!("{:?}", t) }

struct Plugin {
    name: &'static str,
    init: Option<extern "C" fn(i64) -> i64>,
    run: unsafe fn(i64) -> i64,
}

fn main() {
    let items: Vec<Box<dyn Callable>> = vec![
        Box::new(sq as fn(i64) -> i64),
        Box::new(unchecked_half as unsafe fn(i64) -> i64),
        Box::new(c_neg as extern "C" fn(i64) -> i64),
        Box::new(c_twice as unsafe extern "C" fn(i64) -> i64),
        Box::new(sq as unsafe fn(i64) -> i64),
        Box::new(c_neg as unsafe extern "C" fn(i64) -> i64),
    ];
    for it in &items {
        println!("{:<22} {}", it.kind(), it.invoke(12));
    }

    println!("arity {} {} {} {}",
        arity(sq as fn(i64) -> i64),
        arity((|| 1u8) as fn() -> u8),
        arity(choose('+')),
        arity(c_neg as extern "C" fn(i64) -> i64));

    let c = compose(sq, |x| x + 1);
    println!("compose {}", c(5));
    println!("unsafe {:?}", apply_unsafe(&[unchecked_half, sq, |x| x - 1, choose_unsafe(2)], 9));
    for op in ['+', '-', '*', '?'] {
        println!("{} -> {}", op, choose(op)(7, 3));
    }
    for k in 0..3 {
        println!("choose_unsafe {} {}", k, unsafe { choose_unsafe(k)(10) });
    }

    let plugins = [
        Plugin { name: "a", init: Some(c_neg), run: sq },
        Plugin { name: "b", init: None, run: unchecked_half },
        Plugin { name: "c", init: Some(c_neg), run: |x| x * 100 },
    ];
    for p in &plugins {
        let start = p.init.map_or(1, |f| f(3));
        println!("{} {} {}", p.name, start, unsafe { (p.run)(start) });
    }

    // fn pointers as map keys and values
    let mut by_ptr: HashMap<unsafe fn(i64) -> i64, &str> = HashMap::new();
    by_ptr.insert(unchecked_half, "half");
    by_ptr.insert(sq, "sq");
    by_ptr.insert(unchecked_half, "half again");
    let mut names: Vec<_> = by_ptr.values().copied().collect();
    names.sort();
    println!("{:?}", names);

    let mut table: HashMap<&str, extern "C" fn(i64) -> i64> = HashMap::new();
    table.insert("neg", c_neg);
    println!("{}", table["neg"](8));

    // Debug output of fn pointers is an address
    println!("{}", show(c_neg as extern "C" fn(i64) -> i64).starts_with("0x"));
    println!("{}", show(Some(sq as fn(i64) -> i64)).starts_with("Some(0x"));
    let f1: fn(i64) -> i64 = sq;
    let f2 = f1;
    println!("{} {}", f1 == f2, std::ptr::fn_addr_eq(f1, f2));
    let u1: unsafe fn(i64) -> i64 = f1;
    println!("{}", std::ptr::fn_addr_eq(f1, u1));
    println!("{}", std::mem::size_of::<Option<unsafe extern "C" fn(i64) -> i64>>());
    println!("{}", std::any::type_name::<Option<unsafe fn(i64) -> i64>>());
}
