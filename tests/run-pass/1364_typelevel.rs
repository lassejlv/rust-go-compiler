// Type-level programming in everyday disguise: a units-of-measure library
// using PhantomData + marker traits + associated consts, generic numeric
// code over a Num trait with HRTB closures, a plugin registry keyed by
// TypeId with Any downcasting, trait objects with generic default methods,
// supertrait upcasting, and blanket impls over references and Box.
use std::any::{Any, TypeId};
use std::collections::{BTreeMap, HashMap};
use std::fmt::{self, Debug, Display};
use std::marker::PhantomData;
use std::ops::{Add, Div, Mul, Sub};

// ---------------- units ----------------
trait Unit: Copy + Default + Debug {
    const SYMBOL: &'static str;
    const TO_BASE: f64;
}
#[derive(Clone, Copy, Default, Debug)]
struct Meters;
#[derive(Clone, Copy, Default, Debug)]
struct Feet;
#[derive(Clone, Copy, Default, Debug)]
struct Seconds;
#[derive(Clone, Copy, Default, Debug)]
struct Hours;
impl Unit for Meters {
    const SYMBOL: &'static str = "m";
    const TO_BASE: f64 = 1.0;
}
impl Unit for Feet {
    const SYMBOL: &'static str = "ft";
    const TO_BASE: f64 = 0.3048;
}
impl Unit for Seconds {
    const SYMBOL: &'static str = "s";
    const TO_BASE: f64 = 1.0;
}
impl Unit for Hours {
    const SYMBOL: &'static str = "h";
    const TO_BASE: f64 = 3600.0;
}

#[derive(Clone, Copy, Default, Debug)]
struct Per<A, B>(PhantomData<(A, B)>);
impl<A: Unit, B: Unit> Unit for Per<A, B> {
    const SYMBOL: &'static str = "ratio";
    const TO_BASE: f64 = A::TO_BASE / B::TO_BASE;
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
struct Q<U: Unit> {
    v: f64,
    _u: PhantomData<U>,
}

impl<U: Unit> Q<U> {
    fn new(v: f64) -> Self {
        Q { v, _u: PhantomData }
    }
    fn to<V: Unit>(self) -> Q<V> {
        Q::new(self.v * U::TO_BASE / V::TO_BASE)
    }
}
impl<U: Unit> Add for Q<U> {
    type Output = Q<U>;
    fn add(self, o: Self) -> Self {
        Q::new(self.v + o.v)
    }
}
impl<U: Unit> Sub for Q<U> {
    type Output = Q<U>;
    fn sub(self, o: Self) -> Self {
        Q::new(self.v - o.v)
    }
}
impl<U: Unit> Mul<f64> for Q<U> {
    type Output = Q<U>;
    fn mul(self, k: f64) -> Self {
        Q::new(self.v * k)
    }
}
impl<A: Unit, B: Unit> Div<Q<B>> for Q<A> {
    type Output = Q<Per<A, B>>;
    fn div(self, o: Q<B>) -> Q<Per<A, B>> {
        Q::new(self.v / o.v)
    }
}
impl<U: Unit> Display for Q<U> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match f.precision() {
            Some(p) => write!(f, "{:.*} {}", p, self.v, U::SYMBOL),
            None => write!(f, "{} {}", self.v, U::SYMBOL),
        }
    }
}
impl<A: Unit, B: Unit> Q<Per<A, B>> {
    fn describe(&self) -> String {
        format!("{:.3} {}/{}", self.v, A::SYMBOL, B::SYMBOL)
    }
    fn convert<C: Unit, D: Unit>(self) -> Q<Per<C, D>> {
        Q::new(self.v * A::TO_BASE / B::TO_BASE * D::TO_BASE / C::TO_BASE)
    }
}

// ---------------- generic numerics ----------------
trait Num: Copy + PartialOrd + Debug + Add<Output = Self> + Sub<Output = Self> + Mul<Output = Self> {
    const ZERO: Self;
    const ONE: Self;
    fn from_i32(x: i32) -> Self;
}
impl Num for i64 {
    const ZERO: i64 = 0;
    const ONE: i64 = 1;
    fn from_i32(x: i32) -> i64 {
        x as i64
    }
}
impl Num for f64 {
    const ZERO: f64 = 0.0;
    const ONE: f64 = 1.0;
    fn from_i32(x: i32) -> f64 {
        x as f64
    }
}
impl Num for u128 {
    const ZERO: u128 = 0;
    const ONE: u128 = 1;
    fn from_i32(x: i32) -> u128 {
        x as u128
    }
}

fn poly_eval<T: Num>(coeffs: &[T], x: T) -> T {
    coeffs.iter().rev().fold(T::ZERO, |acc, &c| acc * x + c)
}

fn power<T: Num>(mut b: T, mut e: u32) -> T {
    let mut r = T::ONE;
    while e > 0 {
        if e & 1 == 1 {
            r = r * b;
        }
        b = b * b;
        e >>= 1;
    }
    r
}

fn apply_all<T, F>(items: &[T], f: F) -> Vec<String>
where
    F: for<'a> Fn(&'a T) -> &'a dyn Debug,
{
    items.iter().map(|x| format!("{:?}", f(x))).collect()
}

fn pick<'a, T: Num>(xs: &'a [T], pred: impl Fn(&T) -> bool) -> Option<&'a T> {
    xs.iter().filter(|x| pred(x)).fold(None, |best: Option<&'a T>, x| match best {
        Some(b) if *b >= *x => Some(b),
        _ => Some(x),
    })
}

// ---------------- plugin registry ----------------
trait Plugin: Any + Debug {
    fn name(&self) -> &'static str;
    fn run(&mut self, input: &str) -> String;
    fn as_any(&self) -> &dyn Any;
}

#[derive(Debug, Default)]
struct Upper {
    calls: u32,
}
#[derive(Debug, Default)]
struct Counter {
    seen: BTreeMap<char, u32>,
}
#[derive(Debug)]
struct Rot(u8);

impl Plugin for Upper {
    fn name(&self) -> &'static str {
        "upper"
    }
    fn run(&mut self, input: &str) -> String {
        self.calls += 1;
        input.to_uppercase()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
impl Plugin for Counter {
    fn name(&self) -> &'static str {
        "counter"
    }
    fn run(&mut self, input: &str) -> String {
        for c in input.chars().filter(|c| c.is_alphabetic()) {
            *self.seen.entry(c.to_ascii_lowercase()).or_insert(0) += 1;
        }
        input.to_string()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
impl Plugin for Rot {
    fn name(&self) -> &'static str {
        "rot"
    }
    fn run(&mut self, input: &str) -> String {
        input
            .chars()
            .map(|c| match c {
                'a'..='z' => ((c as u8 - b'a' + self.0) % 26 + b'a') as char,
                'A'..='Z' => ((c as u8 - b'A' + self.0) % 26 + b'A') as char,
                _ => c,
            })
            .collect()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Default)]
struct Registry {
    plugins: Vec<Box<dyn Plugin>>,
    by_type: HashMap<TypeId, usize>,
}

impl Registry {
    fn register<P: Plugin + 'static>(&mut self, p: P) -> &mut Self {
        self.by_type.insert(TypeId::of::<P>(), self.plugins.len());
        self.plugins.push(Box::new(p));
        self
    }
    fn get<P: Plugin + 'static>(&self) -> Option<&P> {
        let i = *self.by_type.get(&TypeId::of::<P>())?;
        self.plugins[i].as_any().downcast_ref::<P>()
    }
    fn pipeline(&mut self, input: &str) -> String {
        self.plugins.iter_mut().fold(input.to_string(), |acc, p| p.run(&acc))
    }
}

// ---------------- blanket impls ----------------
trait Describe {
    fn describe(&self) -> String;
}
impl Describe for i32 {
    fn describe(&self) -> String {
        format!("int {}", self)
    }
}
impl Describe for str {
    fn describe(&self) -> String {
        format!("str {:?}", self)
    }
}
impl<T: Describe + ?Sized> Describe for &T {
    fn describe(&self) -> String {
        format!("&{}", (**self).describe())
    }
}
impl<T: Describe + ?Sized> Describe for Box<T> {
    fn describe(&self) -> String {
        format!("Box<{}>", (**self).describe())
    }
}
impl<T: Describe> Describe for [T] {
    fn describe(&self) -> String {
        format!("[{}]", self.iter().map(|x| x.describe()).collect::<Vec<_>>().join(", "))
    }
}
impl<T: Describe, const N: usize> Describe for [T; N] {
    fn describe(&self) -> String {
        format!("array{}{}", N, self[..].describe())
    }
}
impl<A: Describe, B: Describe> Describe for (A, B) {
    fn describe(&self) -> String {
        format!("({} & {})", self.0.describe(), self.1.describe())
    }
}

trait Shape: Debug {
    fn area(&self) -> f64;
    fn scaled(&self, k: f64) -> Box<dyn Shape>;
    fn bigger_than(&self, other: &dyn Shape) -> bool {
        self.area() > other.area()
    }
}
trait Solid: Shape {
    fn volume(&self, depth: f64) -> f64 {
        self.area() * depth
    }
}
#[derive(Debug)]
struct Sq(f64);
#[derive(Debug)]
struct Circ(f64);
impl Shape for Sq {
    fn area(&self) -> f64 {
        self.0 * self.0
    }
    fn scaled(&self, k: f64) -> Box<dyn Shape> {
        Box::new(Sq(self.0 * k))
    }
}
impl Shape for Circ {
    fn area(&self) -> f64 {
        3.0 * self.0 * self.0
    }
    fn scaled(&self, k: f64) -> Box<dyn Shape> {
        Box::new(Circ(self.0 * k))
    }
}
impl Solid for Sq {}
impl Solid for Circ {
    fn volume(&self, depth: f64) -> f64 {
        self.area() * depth / 3.0
    }
}

fn total_area(shapes: &[&dyn Shape]) -> f64 {
    shapes.iter().map(|s| s.area()).sum()
}

fn main() {
    let run: Q<Meters> = Q::new(5000.0);
    let more: Q<Feet> = Q::new(1000.0);
    let total = run + more.to::<Meters>();
    let time: Q<Hours> = Q::new(0.5);
    let speed = total / time.to::<Seconds>();
    println!("{} + {} = {:.2}", run, more, total);
    println!("speed {} = {}", speed.describe(), speed.convert::<Feet, Hours>().describe());
    println!("{:.1} {:?}", Q::<Feet>::new(3.0).to::<Meters>() * 2.0, (Q::<Seconds>::new(1.0) - Q::new(0.25)).v);
    println!("{} {}", <Per<Meters, Seconds> as Unit>::TO_BASE, <Per<Feet, Hours>>::TO_BASE * 1e6);
    println!("{}", Q::<Meters>::new(1.0).v < Q::<Meters>::new(2.0).v);

    println!("{} {} {}", poly_eval(&[1i64, 2, 3], 10), poly_eval(&[0.5, 0.25], 2.0), poly_eval(&[1u128, 0, 1], 1 << 40));
    println!("{} {} {:?}", power(3i64, 13), power(1.5f64, 4), power(3u128, 40));
    println!("{:?}", apply_all(&[(1, 'a'), (2, 'b')], |p| &p.1));
    println!("{:?} {:?} {:?}", pick(&[3i64, 9, 4, 9, 1], |x| x % 3 == 0), pick(&[0.5f64, -2.0], |x| *x < 0.0), pick::<i64>(&[], |_| true));
    println!("{:?}", [1, 2, 3].map(Num::from_i32).iter().map(|x: &f64| x / 2.0).collect::<Vec<_>>());

    let mut reg = Registry::default();
    reg.register(Counter::default()).register(Rot(13)).register(Upper::default());
    println!("{}", reg.pipeline("Hello, World"));
    println!("{}", reg.pipeline("Uryyb ntnva"));
    println!("upper calls {:?}", reg.get::<Upper>().map(|u| u.calls));
    println!("letters {:?}", reg.get::<Counter>().map(|c| c.seen.iter().filter(|(_, &n)| n > 1).map(|(c, n)| format!("{}{}", c, n)).collect::<String>()));
    println!("rot {:?} missing {:?}", reg.get::<Rot>().map(|r| r.0), reg.get::<Registry2>().is_some());
    println!("{:?}", reg.plugins.iter().map(|p| p.name()).collect::<Vec<_>>());

    println!("{}", 5.describe());
    println!("{}", (&&7).describe());
    println!("{}", Box::new("hi").describe());
    let b: Box<str> = "boxed".into();
    println!("{}", b.describe());
    println!("{}", [1, 2, 3].describe());
    println!("{}", (4, "x").describe());
    println!("{}", vec![Box::new(1), Box::new(2)][..].describe());

    let s = Sq(2.0);
    let c = Circ(1.5);
    let shapes: Vec<&dyn Shape> = vec![&s, &c];
    println!("total {} sq>circ {} circ>sq {}", total_area(&shapes), s.bigger_than(&c), c.bigger_than(&s));
    let solids: Vec<Box<dyn Solid>> = vec![Box::new(Sq(1.0)), Box::new(Circ(1.0))];
    for so in &solids {
        let as_shape: &dyn Shape = so.as_ref();
        println!("{:?} vol {} scaled {:?} area {}", so, so.volume(2.0), as_shape.scaled(2.0), as_shape.scaled(0.5).area());
    }
    let upcast: Vec<Box<dyn Shape>> = solids.into_iter().map(|s| s as Box<dyn Shape>).collect();
    println!("{:?}", upcast.iter().map(|s| s.area()).collect::<Vec<_>>());
}

#[derive(Debug)]
struct Registry2;
impl Plugin for Registry2 {
    fn name(&self) -> &'static str {
        "r2"
    }
    fn run(&mut self, i: &str) -> String {
        i.into()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
