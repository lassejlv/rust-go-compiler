use std::marker::PhantomData;
use std::ops::{Add, Sub, Mul, Div};
use std::fmt;

// Dimension exponents encoded as types: Length^L Time^T Mass^M via traits with consts
trait Dim { const L: i32; const T: i32; const M: i32; }
#[derive(Debug, Clone, Copy)] struct Dl<const L: i32, const T: i32, const M: i32>;
impl<const L: i32, const T: i32, const M: i32> Dim for Dl<L, T, M> { const L: i32 = L; const T: i32 = T; const M: i32 = M; }

#[derive(Clone, Copy)]
struct Q<D: Dim> { v: f64, _d: PhantomData<D> }
impl<D: Dim> Q<D> { fn new(v: f64) -> Self { Q { v, _d: PhantomData } } }

type Meters = Dl<1, 0, 0>;
type Seconds = Dl<0, 1, 0>;
type Kg = Dl<0, 0, 1>;
type Speed = Dl<1, -1, 0>;
type Accel = Dl<1, -2, 0>;
type Newton = Dl<1, -2, 1>;
type Scalar = Dl<0, 0, 0>;

impl<D: Dim> Add for Q<D> { type Output = Q<D>; fn add(self, o: Self) -> Self { Q::new(self.v + o.v) } }
impl<D: Dim> Sub for Q<D> { type Output = Q<D>; fn sub(self, o: Self) -> Self { Q::new(self.v - o.v) } }
impl<D: Dim> Mul<f64> for Q<D> { type Output = Q<D>; fn mul(self, k: f64) -> Self { Q::new(self.v * k) } }

// Concrete dimension arithmetic (no generic const exprs on stable): enumerate needed combos via a trait
trait DimMul<Rhs> { type Out: Dim; }
trait DimDiv<Rhs> { type Out: Dim; }
impl DimDiv<Seconds> for Meters { type Out = Speed; }
impl DimDiv<Seconds> for Speed { type Out = Accel; }
impl DimMul<Accel> for Kg { type Out = Newton; }
impl DimMul<Seconds> for Speed { type Out = Meters; }
impl DimMul<Seconds> for Accel { type Out = Speed; }
impl DimDiv<Meters> for Meters { type Out = Scalar; }

impl<A: Dim + DimMul<B>, B: Dim> Mul<Q<B>> for Q<A> {
    type Output = Q<<A as DimMul<B>>::Out>;
    fn mul(self, o: Q<B>) -> Self::Output { Q::new(self.v * o.v) }
}
impl<A: Dim + DimDiv<B>, B: Dim> Div<Q<B>> for Q<A> {
    type Output = Q<<A as DimDiv<B>>::Out>;
    fn div(self, o: Q<B>) -> Self::Output { Q::new(self.v / o.v) }
}

fn unit_str<D: Dim>() -> String {
    let mut parts = vec![];
    for (name, e) in [("m", D::L), ("s", D::T), ("kg", D::M)] {
        match e { 0 => {}, 1 => parts.push(name.to_string()), _ => parts.push(format!("{}^{}", name, e)) }
    }
    if parts.is_empty() { "1".to_string() } else { parts.join("*") }
}
impl<D: Dim> fmt::Display for Q<D> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "{:.3} {}", self.v, unit_str::<D>()) }
}

fn total<D: Dim>(xs: &[Q<D>]) -> Q<D> { xs.iter().fold(Q::new(0.0), |a, b| a + Q::new(b.v)) }

fn main() {
    let d: Q<Meters> = Q::new(100.0);
    let t: Q<Seconds> = Q::new(9.58);
    let v = d / t;
    println!("speed {}", v);
    let a = v / Q::<Seconds>::new(2.0);
    println!("accel {}", a);
    let f = Q::<Kg>::new(80.0) * a;
    println!("force {}", f);
    println!("back {}", (a * Q::<Seconds>::new(2.0)) * t);
    println!("ratio {}", d / Q::<Meters>::new(25.0));
    println!("sum {}", total(&[d, d * 0.5, Q::new(1.0)]));
    println!("diff {}", d - Q::new(0.5));
    println!("{} {} {}", unit_str::<Newton>(), unit_str::<Scalar>(), unit_str::<Dl<-3, 2, 0>>());
}
