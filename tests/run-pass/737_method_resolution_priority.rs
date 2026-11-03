// Method resolution priority: inherent vs trait methods, by-value vs autoref
// receivers, impls for T vs &T vs &mut T vs Box<T>, deref chains, trait
// methods shadowed by inherent ones, and UFCS disambiguation.
use std::fmt;
use std::ops::Deref;
use std::rc::Rc;

trait Describe { fn describe(&self) -> String; }
trait Name { fn name(&self) -> String { "Name::name default".into() } }
trait Name2 { fn name(&self) -> String { "Name2::name default".into() } }

struct Thing(i32);

impl Thing {
    fn describe(&self) -> String { format!("inherent Thing({})", self.0) }
    fn consume(self) -> String { "inherent consume(self)".into() }
}
impl Describe for Thing { fn describe(&self) -> String { format!("trait Thing({})", self.0) } }
impl Name for Thing {}
impl Name2 for Thing { fn name(&self) -> String { "Name2 for Thing".into() } }

impl Describe for &Thing { fn describe(&self) -> String { "trait for &Thing".into() } }
impl Describe for Box<Thing> { fn describe(&self) -> String { "trait for Box<Thing>".into() } }

trait Consume { fn consume(self) -> String; }
impl Consume for &Thing { fn consume(self) -> String { "Consume for &Thing".into() } }

trait ByRef { fn which(&self) -> &'static str; }
trait ByMut { fn which(&mut self) -> &'static str; }
trait ByVal { fn which(self) -> &'static str; }
struct R;
impl ByRef for R { fn which(&self) -> &'static str { "&self" } }
impl ByVal for &&R { fn which(self) -> &'static str { "self on &&R" } }

struct Inner { v: Vec<i32> }
impl Inner { fn len(&self) -> usize { 1000 + self.v.len() } }
struct Outer { inner: Inner }
impl Deref for Outer { type Target = Inner; fn deref(&self) -> &Inner { &self.inner } }
impl Deref for Inner { type Target = Vec<i32>; fn deref(&self) -> &Vec<i32> { &self.v } }

struct Shown(u8);
impl fmt::Display for Shown { fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "Display({})", self.0) } }
impl Shown { #[allow(dead_code)] fn to_string_custom(&self) -> String { "custom".into() } }

trait Area { const SIDES: u32; fn area(&self) -> f64; fn sides(&self) -> u32 { Self::SIDES } }
trait Perimeter { const SIDES: u32; fn sides(&self) -> u32 { Self::SIDES * 10 } }
struct Tri;
impl Area for Tri { const SIDES: u32 = 3; fn area(&self) -> f64 { 0.5 } }
impl Perimeter for Tri { const SIDES: u32 = 30; }

trait Greet { fn hi(&self) -> String; }
impl<T: fmt::Debug> Greet for T { fn hi(&self) -> String { format!("hi {:?}", self) } }

#[derive(Debug, Clone)]
struct Cl(u8);
fn clone_kinds<'a>(x: &Cl, y: &'a &'a Cl) -> (Cl, &'a Cl, Cl) {
    let a = x.clone();     // Cl::clone via autoderef
    let b = (*y).clone();  // &Cl clone -> &Cl? no: (*y) is &Cl, clone gives Cl
    let c = Clone::clone(*y);
    let _ = b;
    (a, *y, c)
}

fn main() {
    let t = Thing(7);
    println!("1 {}", t.describe());
    println!("2 {}", Describe::describe(&t));
    println!("3 {}", <&Thing as Describe>::describe(&&t));
    println!("4 {}", (&t).describe());
    println!("5 {}", (&&t).describe());
    let bx = Box::new(Thing(9));
    println!("6 {}", bx.describe());
    println!("7 {}", Describe::describe(&bx));
    println!("8 {}", <Thing as Name>::name(&t));
    println!("9 {}", Name2::name(&t));
    println!("10 {}", (&t).consume());
    println!("11 {}", t.consume());

    let r = R;
    println!("12 {}", r.which());
    println!("13 {}", (&r).which());
    println!("14 {}", (&&r).which());
    println!("15 {}", ByRef::which(&r));

    let o = Outer { inner: Inner { v: vec![1, 2, 3] } };
    println!("16 len via deref: {}", o.len());
    println!("17 vec len: {}", o.inner.v.len());
    println!("18 deref twice: {} first={:?}", (**o).len(), o.first());
    println!("19 iter sum: {}", o.iter().sum::<i32>());

    let s = Shown(4);
    println!("20 {} {}", s.to_string(), s.to_string_custom());
    println!("21 sides {} {} {}", Area::sides(&Tri), Perimeter::sides(&Tri), <Tri as Area>::SIDES + <Tri as Perimeter>::SIDES);
    println!("22 {} {} {}", 5.hi(), "x".hi(), (&Some(1)).hi());

    let rc = Rc::new(vec![String::from("a"), String::from("bb")]);
    let rc2 = rc.clone();
    let inner_clone: Vec<String> = (*rc).clone();
    println!("23 rc strong={} len={} {:?}", Rc::strong_count(&rc2), rc.len(), inner_clone);
    let (a, b, c) = clone_kinds(&Cl(1), &&Cl(2));
    println!("24 {:?} {:?} {:?}", a, b, c);
    let nested: Option<&Option<i32>> = Some(&Some(3));
    println!("25 {:?} {:?}", nested.cloned(), nested.copied().flatten());
    let s2 = String::from("hello");
    let r1: &str = &s2;
    println!("26 {} {} {}", r1.len(), str::len(&s2), <String>::len(&s2));
    let mut v = vec![3, 1, 2];
    (&mut v).sort();
    println!("27 {:?} {}", v, <[i32]>::len(&v));
}
