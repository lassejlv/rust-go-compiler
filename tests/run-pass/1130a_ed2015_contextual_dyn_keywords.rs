// edition:2015
// Edition 2015: async/await/dyn/try as field, module, fn and macro-argument
// names; `dyn` stays a contextual keyword in types (dyn Trait, &dyn Tr + 'a,
// dyn Fn(..)); anonymous trait method parameters.
use std::fmt::Display;
struct S { async: i32, await: i32, dyn: i32, try: i32 }
mod dyn { pub fn f() -> i32 { 5 } }
fn async(x: i32) -> i32 { x + 1 }
trait Tr { fn show(&self) -> String; fn add(&self, i32, &str) -> String { self.show() + "!" } }
impl Tr for i32 { fn show(&self) -> String { format!("i{}", self) } }
fn boxed() -> Box<dyn Tr> { Box::new(3) }
fn refd(x: &dyn Display) -> String { format!("{}", x) }
fn bare(x: &Display) -> String { format!("<{}>", x) }
fn lt<'a>(x: &'a (dyn Tr + 'a)) -> String { x.show() }
macro_rules! mk { ($i:ident) => { stringify!($i) } }
fn main() {
    let s = S { async: 1, await: 2, dyn: 3, try: 4 };
    println!("{} {} {} {}", s.async, s.await, s.dyn, s.try);
    println!("{} {}", dyn::f(), async(1));
    println!("{} {} {} {}", boxed().show(), refd(&7), bare(&8), lt(&9));
    println!("{} {}", mk!(async), stringify!(dyn try await));
    let v: Vec<Box<dyn Fn(i32) -> i32>> = vec![Box::new(|x| x + 1)];
    println!("{} {}", v[0](1), 5.add(1, "x"));
}
