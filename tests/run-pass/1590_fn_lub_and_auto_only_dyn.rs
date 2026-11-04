// Distinct fn items / non-capturing closures in if/else and match arms meet
// at a fn pointer; `dyn Send` / `dyn Send + Sync` trait objects with only
// auto traits.
use std::fmt::Debug;

fn fn_a(x: i32) -> i32 { x * 10 }
fn fn_b(x: i32) -> i32 { x * 100 }
unsafe fn u(x: i32) -> i32 { x + 1 }
fn s(x: i32) -> i32 { x + 2 }
fn id<T>(x: T) -> T { x }

fn pick(c: bool) -> fn(i32) -> i32 {
    let f = if c { |x: i32| x + 1 } else { |x: i32| x * 2 };
    f
}

fn size(x: &dyn Send) -> usize { std::mem::size_of_val(x) }

struct D(&'static str);
impl Drop for D {
    fn drop(&mut self) { println!("drop {}", self.0); }
}

fn main() {
    let c = std::env::args().count() > 5;
    println!("{} {}", pick(true)(5), pick(false)(5));
    let g = match 3 { 1 => |x: i32| x, _ => |x: i32| -x };
    println!("{}", g(4));
    let h = if c { fn_a } else { fn_b };
    println!("{}", h(1));
    let k = match 2 { 1 => fn_a, 2 => fn_b, _ => |x| x };
    println!("{}", k(7));
    let f = if c { u } else { s };
    println!("{}", unsafe { f(1) });
    let m = match 1 { 0 => id, _ => |x: u8| x };
    println!("{}", m(9));
    let n = match 5 { 0 => fn_a, 1 => panic!(), _ => fn_b };
    println!("{}", n(2));

    let v: &dyn Send = &5u64;
    println!("{}", size(v));
    let b: Box<dyn Sync + Send> = Box::new(1u8);
    println!("{}", std::mem::size_of_val(&*b));
    let bs: Vec<Box<dyn Send>> = vec![Box::new(D("a")), Box::new(3i32), Box::new(D("b"))];
    println!("{}", bs.len());
    drop(bs);
    println!("{}", std::any::type_name::<dyn Send + Sync>());
    println!("{}", std::any::type_name::<Box<dyn Send>>());
    let d: &(dyn Debug + Send) = &1;
    println!("{:?}", d);
}
