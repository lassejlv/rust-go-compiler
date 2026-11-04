use std::fmt::{Debug, Display};
use std::marker::PhantomData;

struct Foo<T>(T);
impl Foo<i32> { fn kind(&self) -> &'static str { "i32" } fn make() -> Self { Foo(-1) } const K: u8 = 1; }
impl Foo<u8> { fn kind(&self) -> &'static str { "u8" } fn make() -> Self { Foo(1) } const K: u8 = 2; }
impl Foo<String> { fn kind(&self) -> &'static str { "string" } }
impl<T> Foo<Vec<T>> { fn kind(&self) -> &'static str { "vec" } }
impl<T> Foo<T> {
    fn show(&self) -> String where T: Display { format!("Foo({})", self.0) }
    fn dbg(&self) -> String where T: Debug { format!("Foo({:?})", self.0) }
    fn inner(self) -> T { self.0 }
}

struct Meters; struct Feet;
struct Len<U> { v: f64, _u: PhantomData<U> }
impl Len<Meters> { fn to_feet(&self) -> Len<Feet> { Len { v: self.v * 3.28084, _u: PhantomData } } fn unit() -> &'static str { "m" } }
impl Len<Feet> { fn to_meters(&self) -> Len<Meters> { Len { v: self.v / 3.28084, _u: PhantomData } } fn unit() -> &'static str { "ft" } }
impl<U> std::ops::Add for Len<U> { type Output = Len<U>; fn add(self, o: Self) -> Self { Len { v: self.v + o.v, _u: PhantomData } } }

#[derive(Debug, PartialEq)]
struct Rgb(u8, u8, u8);
impl From<(u8, u8, u8)> for Rgb { fn from(t: (u8, u8, u8)) -> Self { Rgb(t.0, t.1, t.2) } }
impl From<u32> for Rgb { fn from(x: u32) -> Self { Rgb((x >> 16) as u8, (x >> 8) as u8, x as u8) } }
impl From<&str> for Rgb { fn from(s: &str) -> Self { Rgb::from(u32::from_str_radix(s.trim_start_matches('#'), 16).unwrap()) } }
impl From<Rgb> for u32 { fn from(c: Rgb) -> u32 { (c.0 as u32) << 16 | (c.1 as u32) << 8 | c.2 as u32 } }
impl TryFrom<Vec<u8>> for Rgb {
    type Error = String;
    fn try_from(v: Vec<u8>) -> Result<Self, String> {
        match v[..] { [r, g, b] => Ok(Rgb(r, g, b)), _ => Err(format!("need 3, got {}", v.len())) }
    }
}
fn to_rgb<T: Into<Rgb>>(t: T) -> Rgb { t.into() }

trait Zeroed { fn zeroed() -> Self; }
impl<T: Default, const N: usize> Zeroed for [T; N] { fn zeroed() -> Self { std::array::from_fn(|_| T::default()) } }
impl Zeroed for () { fn zeroed() {} }
impl<A: Zeroed, B: Zeroed> Zeroed for (A, B) { fn zeroed() -> Self { (A::zeroed(), B::zeroed()) } }

fn main() {
    println!("{} {} {} {}", Foo(1i32).kind(), Foo(1u8).kind(), Foo(String::new()).kind(), Foo(vec![1]).kind());
    println!("{} {}", Foo::<i32>::make().0, Foo::<u8>::make().0);
    println!("{} {}", Foo::<i32>::K, Foo::<u8>::K);
    println!("{} {}", Foo(3.5).show(), Foo(vec![Some(1)]).dbg());
    println!("{:?}", Foo(vec!["x"]).inner());
    let a: Len<Meters> = Len { v: 10.0, _u: PhantomData };
    let b = a.to_feet();
    let bv = b.v;
    println!("{:.2}{} {:.2}{}", bv, Len::<Feet>::unit(), (b + Len { v: 1.0, _u: PhantomData }).to_meters().v, Len::<Meters>::unit());
    println!("{:?} {:?} {:?}", to_rgb((1, 2, 3)), to_rgb(0x10203u32), to_rgb("#ff8000"));
    let n: u32 = Rgb(1, 1, 1).into();
    println!("{}", n);
    let r: Result<Rgb, _> = vec![9u8, 8, 7].try_into();
    let e: Result<Rgb, String> = Rgb::try_from(vec![1u8]);
    println!("{:?} {:?}", r, e);
    let z: ([u8; 3], ((), [Option<i8>; 2])) = Zeroed::zeroed();
    println!("{:?}", z);
    let z2 = <[(String, bool); 2]>::zeroed();
    println!("{:?}", z2);
}
