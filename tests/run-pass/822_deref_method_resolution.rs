use std::fmt::Display;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

trait Describe { fn describe(&self) -> String; }
impl<T: Display> Describe for T { fn describe(&self) -> String { format!("disp({})", self) } }

trait Speak { fn speak(&self) -> String { "generic speak".into() } fn name(&self) -> &'static str; }
struct Dog;
impl Dog { fn speak(&self) -> String { "inherent woof".into() } }
impl Speak for Dog { fn speak(&self) -> String { "trait woof".into() } fn name(&self) -> &'static str { "dog" } }
impl Speak for &Dog { fn name(&self) -> &'static str { "&dog" } }
struct Cat;
impl Speak for Cat { fn name(&self) -> &'static str { "cat" } }

struct Wrapper<T> { inner: T, reads: std::cell::Cell<u32> }
impl<T> Deref for Wrapper<T> { type Target = T; fn deref(&self) -> &T { self.reads.set(self.reads.get() + 1); &self.inner } }
impl<T> DerefMut for Wrapper<T> { fn deref_mut(&mut self) -> &mut T { &mut self.inner } }
impl<T> Wrapper<T> { fn new(inner: T) -> Self { Wrapper { inner, reads: std::cell::Cell::new(0) } } fn len(&self) -> usize { 999 } }

struct Meters(f64);
impl Deref for Meters { type Target = f64; fn deref(&self) -> &f64 { &self.0 } }

trait Area { fn area(&self) -> f64; }
impl Area for f64 { fn area(&self) -> f64 { self * self } }
impl Area for Box<dyn Area> { fn area(&self) -> f64 { -1.0 } }

struct Stack { items: Vec<i32> }
impl Deref for Stack { type Target = Vec<i32>; fn deref(&self) -> &Vec<i32> { &self.items } }
impl DerefMut for Stack { fn deref_mut(&mut self) -> &mut Vec<i32> { &mut self.items } }

trait Twice { fn twice(self) -> Self; }
impl Twice for i32 { fn twice(self) -> i32 { self * 2 } }
impl Twice for String { fn twice(self) -> String { self.repeat(2) } }
impl<T: Twice + Clone> Twice for Vec<T> { fn twice(self) -> Vec<T> { self.into_iter().map(Twice::twice).collect() } }

trait Shout { fn shout(&self) -> String; }
impl Shout for str { fn shout(&self) -> String { self.to_uppercase() + "!" } }
impl Shout for [i32] { fn shout(&self) -> String { format!("{} items!", self.len()) } }

fn takes_str(s: &str) -> usize { s.len() }
fn takes_slice(s: &[i32]) -> i32 { s.iter().sum() }

fn main() {
    println!("{} {} {}", 5.describe(), "hi".describe(), 2.5f32.describe());
    let d = Dog;
    println!("{} | {} | {} | {}", d.speak(), Speak::speak(&d), <&Dog as Speak>::speak(&&d), (&d).name());
    println!("{} {}", <Dog as Speak>::name(&d), <&Dog as Speak>::name(&&d));
    println!("{} {}", Cat.speak(), Cat.name());
    let mut w = Wrapper::new(vec![3, 1, 2]);
    println!("len via inherent {} len via deref {}", w.len(), (*w).len());
    w.push(0);
    w.sort();
    println!("{:?} first={:?} contains={} reads={}", *w, w.first(), w.contains(&2), w.reads.get());
    let ws = Wrapper::new(String::from("hello"));
    println!("{} {} {}", ws.to_uppercase(), takes_str(&ws), ws.starts_with("he"));
    let m = Meters(3.0);
    println!("{} {} {}", m.sqrt(), m.area(), (*m).max(10.0));
    let b: Box<dyn Area> = Box::new(4.0f64);
    println!("{} {}", b.area(), (*b).area());
    let rc = Rc::new(Wrapper::new(vec![1, 2, 3]));
    println!("{} {}", rc.len(), rc.iter().sum::<i32>());
    let mut st = Stack { items: vec![] };
    st.push(1); st.push(2); st.extend([3, 4]);
    let top = st.pop();
    println!("{:?} {:?} len={} {}", top, st.items, st.len(), takes_slice(&st));
    println!("{} {:?} {:?}", 21.twice(), String::from("ab").twice(), vec![1, 2].twice());
    println!("{:?}", vec![String::from("x"), String::from("yz")].twice());
    println!("{} {}", "hey".shout(), String::from("own").shout());
    let v = vec![1, 2, 3];
    println!("{} {}", v.shout(), [5, 6][..].shout());
    let boxed_str: Box<str> = "boxed".into();
    println!("{} {}", boxed_str.shout(), takes_str(&boxed_str));
    let nested: Box<Box<Vec<i32>>> = Box::new(Box::new(vec![7, 8]));
    println!("{} {:?}", nested.len(), nested.iter().rev().collect::<Vec<_>>());
    let rr: &&&i32 = &&&5;
    println!("{} {} {}", rr.pow(2), rr.describe(), **rr + 1);
    let s = String::from("x");
    let rs = &s;
    println!("{} {}", rs.describe(), (&rs).len());
    let opt: Option<&String> = Some(&s);
    println!("{:?}", opt.map(|s| s.len()));
    let cl = |x: &dyn Describe| x.describe();
    println!("{} {}", cl(&1u8), cl(&"z"));
}
