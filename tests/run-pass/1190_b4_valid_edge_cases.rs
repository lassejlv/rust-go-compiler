// Valid programs near the checks added for invalid ones: they must keep
// compiling (late-bound unused lifetimes in impl methods, glob imports of
// visible items, ?Sized generics, Drop impls repeating the type's bounds,
// bounded ADTs, explicit self types, allowed overflowing literals, uncalled
// generic functions that borrow-check, unit struct assignment).
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::rc::Rc;

trait Lookup {
    fn get<'a>(&self, x: &'a u8) -> &'a u8;
}
struct L;
impl Lookup for L {
    // 'b is unused, hence late-bound: same early-bound count as the trait
    fn get<'a, 'b>(&self, x: &'a u8) -> &'a u8 {
        x
    }
}

mod shapes {
    fn helper() -> i32 { 1 }
    pub(crate) fn area() -> i32 { 2 }
    pub mod inner {
        use super::*;
        pub fn total() -> i32 { helper() + area() }
    }
}
use shapes::*;

fn show<T: ?Sized + Debug>(x: &T) -> String { format!("{:?}", x) }

struct Guard<T: Debug>(T);
impl<T: Debug> Drop for Guard<T> {
    fn drop(&mut self) { println!("drop {:?}", self.0); }
}

struct Cache<K: Hash + Eq, V> { m: HashMap<K, V> }

struct Node { v: i32 }
impl Node {
    fn by_rc(self: Rc<Self>) -> i32 { self.v }
    fn by_box(self: Box<Self>) -> i32 { self.v + 1 }
}

#[allow(overflowing_literals)]
fn wrapped() -> u8 { let x: u8 = 0x1FF; x }

fn pair<T: Copy>(x: T) -> (T, T) { (x, x) }
fn swap_first_last<T>(v: &mut Vec<T>) { let n = v.len(); if n > 1 { v.swap(0, n - 1); } }
fn apply<F: FnMut(&mut Vec<i32>)>(v: &mut Vec<i32>, mut f: F) { f(v); f(v); }
fn never_called<T: Clone>(v: &mut Vec<T>) -> Option<T> { let x = v.pop()?; v.push(x.clone()); Some(x) }

struct Unit;
enum Dir { North, South }

fn main() {
    let a = 7u8;
    println!("{}", L.get(&a));
    println!("{}", inner::total() + area());
    println!("{} {}", show("str"), show(&[1, 2][..]));
    let _g = Guard(3);
    let mut c: Cache<String, u8> = Cache { m: HashMap::new() };
    c.m.insert("k".to_string(), 1);
    println!("{}", c.m.len());
    println!("{} {}", Rc::new(Node { v: 4 }).by_rc(), Box::new(Node { v: 4 }).by_box());
    println!("{}", wrapped());
    println!("{:?}", pair(5));
    let mut v = vec![1, 2, 3];
    swap_first_last(&mut v);
    apply(&mut v, |w| w.push(9));
    println!("{:?}", v);
    let mut s = vec![String::from("x")];
    let _ = &never_called::<String>;
    s.push(String::new());
    let u;
    (u, Unit) = (1, Unit);
    let d = Dir::North;
    let name = match d { Dir::North => "north", Dir::South => "south" };
    println!("{} {} {}", u, s.len(), name);
    let neg: i8 = -128;
    println!("{}", neg);
}
