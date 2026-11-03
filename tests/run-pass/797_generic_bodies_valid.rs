// Valid generic code: every generic body is also type-checked
// polymorphically (type parameters as placeholders, where-clauses as the
// only facts); none of these may be reported.
#![allow(dead_code, unused)]
mod v1 {
use std::fmt::Display;
use std::ops::{Add, Deref};
use std::collections::HashMap;
use std::hash::Hash;

pub trait Shape { fn area(&self) -> f64; fn name(&self) -> String { format!("shape {}", self.area()) } fn twice(&self) -> f64 { self.area() + self.area() } }
pub trait Num: Copy + Add<Output = Self> + PartialOrd + Default {}
impl Num for i32 {}

pub fn sum<T: Num>(xs: &[T]) -> T { let mut s = T::default(); for &x in xs { s = s + x; } s }
pub fn biggest<T: Num>(a: T, b: T) -> T { if a > b { a } else { b } }
pub fn show<T: Display>(t: T) -> String { t.to_string() }
pub fn show_ref<T: Display + ?Sized>(t: &T) -> String { t.to_string() }
pub fn dlen<D: Deref<Target = str>>(d: D) -> usize { d.len() }
pub fn dlen2<D: Deref>(d: D) -> String where D::Target: Display { d.to_string() }
pub fn count<K: Hash + Eq + Clone>(ks: &[K]) -> HashMap<K, usize> { let mut m = HashMap::new(); for k in ks { *m.entry(k.clone()).or_insert(0) += 1; } m }
pub fn conv<T: From<u8>>() -> T { T::from(3u8) }
pub fn conv2<T: Into<u64>>(t: T) -> u64 { t.into() }
pub fn ident<T>(t: T) -> T { let u: T = t; u }
pub fn cl<T: Clone>(t: &T) -> T { t.clone() }
pub fn refclone<T>(t: &T) -> &T { t.clone() }
pub fn it<I: Iterator<Item = u32>>(i: I) -> u32 { i.map(|x| x + 1).sum() }
pub fn it2<I: IntoIterator>(i: I) -> usize where I::Item: Display { i.into_iter().map(|x| x.to_string()).count() }
pub fn dbl<T: std::ops::Mul<Output = T> + Copy>(x: T) -> T { x * x }
pub fn addref<'a, T>(a: &'a T, b: &'a T) -> T where &'a T: Add<&'a T, Output = T> { a + b }
pub fn mx<T: Ord>(a: T, b: T) -> T { a.max(b) }
pub fn eqs<T: Eq>(a: &T, b: &T) -> bool { a == b }
pub fn ptr<T>(a: &T, b: &T) -> bool { std::ptr::eq(a, b) }
pub fn anyid<T: 'static>(_t: &T) -> std::any::TypeId { std::any::TypeId::of::<T>() }
pub fn bx<T>(t: T) -> Box<T> { Box::new(t) }
pub fn opt<T>(t: Option<T>) -> T where T: Default { t.unwrap_or_default() }
pub fn never<T>() -> T { panic!("x") }
pub fn lp<T>() -> T { loop {} }
pub fn errfn<T, E: std::fmt::Debug>(r: Result<T, E>) -> T { r.unwrap() }
pub fn asref<T: AsRef<str>>(t: T) -> usize { t.as_ref().len() }
pub fn borrow<T: std::borrow::Borrow<str>>(t: T) -> usize { t.borrow().len() }
pub fn intoit<T>(v: Vec<T>) -> Vec<T> { v.into_iter().rev().collect() }
pub fn cmpo<T: PartialOrd>(a: &T, b: &T) -> bool { a < b }
pub fn addassign<T: std::ops::AddAssign + Copy>(a: &mut T, b: T) { *a += b; }
pub fn constn<const N: usize>() -> [u8; N] { [0; N] }
pub fn constn2<const N: usize>() -> usize { N + 1 }
pub fn implarg(x: impl Display) -> String { format!("{}", x) }
pub fn implret<T: Clone>(v: &[T]) -> impl Iterator<Item = T> + '_ { v.iter().cloned() }
pub fn closure<F: Fn(i32) -> i32>(f: F) -> i32 { f(1) }
pub fn closure2<T, F: FnOnce() -> T>(f: F) -> T { f() }
pub fn dynr<T: Shape>(t: &T) -> &dyn Shape { t }
pub fn defm<T: Shape>(t: &T) -> String { t.name() }
pub fn defalt<T: Default + PartialEq>(t: &T) -> bool { *t == T::default() }
pub fn tostr<T: ToString>(t: T) -> String { t.to_string() }
pub fn neg<T: std::ops::Neg<Output = T>>(t: T) -> T { -t }
pub fn sz<T>() -> usize { std::mem::size_of::<T>() }
pub fn swap<T>(a: &mut T, b: &mut T) { std::mem::swap(a, b) }
pub fn fmtd<T: std::fmt::Debug>(t: &T) -> String { format!("{:?}", t) }

pub struct W<T>(T);
impl<T: Display> W<T> { fn s(&self) -> String { self.0.to_string() } fn get(self) -> T { self.0 } }
impl<T> Deref for W<T> { type Target = T; fn deref(&self) -> &T { &self.0 } }
impl<T: Add<Output = T>> Add for W<T> { type Output = W<T>; fn add(self, o: W<T>) -> W<T> { W(self.0 + o.0) } }
pub fn wd<T: Display>(w: &W<T>) -> String { w.s() + &(**w).to_string() }
pub trait Animal { fn speak(&self) -> String; fn loud(&self) -> String where Self: Sized { self.speak().to_uppercase() } fn me(self) -> Self where Self: Sized { self } }
pub trait Cnt { type Item; fn first(&self) -> Option<Self::Item>; fn has(&self) -> bool { self.first().is_some() } }
impl<T: Clone> Cnt for Vec<T> { type Item = T; fn first(&self) -> Option<T> { self.get(0).cloned() } }

pub struct S; impl Shape for S { fn area(&self) -> f64 { 1.0 } }
}
mod v2 {
use std::fmt::{self, Display};
use std::ops::Add;

pub trait Ext { fn ext(&self) -> u8 { 1 } }
impl<T: ?Sized> Ext for T {}
pub fn e1<T>(t: T) -> u8 { t.ext() }
pub fn e2<T: Display>(t: &T) -> String { (&t).to_string() }
pub fn e3<T: From<i32>>() -> T { 5.into() }
pub fn e4<T: Into<i64>>(t: T) -> i64 { t.into() + 1 }
pub fn e5<T: Copy>(x: &T) -> T { *x }
pub trait B<X> { fn b(&self) -> X; }
pub trait A<X>: B<X> { fn a(&self) -> X { self.b() } }
pub trait C: A<u8> { fn c(&self) -> u8 { self.b() + self.a() } }
pub fn e6<T: C>(t: &T) -> u8 { t.b() }
pub trait K { const N: usize; fn k(&self) -> usize { Self::N + 1 } }
pub fn e7<T: K>() -> usize { T::N * 2 }
pub fn e8<T: PartialEq + fmt::Debug>(a: T, b: T) { assert_eq!(a, b); assert!(a == b); }
pub fn e9<T: Clone>(t: T, n: usize) -> Vec<T> { vec![t; n] }
pub struct P<T>(T);
impl<T: Display> Display for P<T> { fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "<{}>", self.0) } }
pub fn e10<T: PartialEq<U>, U>(a: T, b: U) -> bool { a == b }
pub fn e11<T: PartialOrd<i32>>(t: T) -> bool { t > 0 }
pub fn e12<T>(t: T, b: bool) -> T { if b { t } else { panic!() } }
pub fn e13<T>(t: T) -> T { let c = |x: T| x; c(t) }
pub fn e14<T: Default>(n: usize) -> Vec<T> { (0..n).map(|_| T::default()).collect() }
pub fn e15<T>(v: &mut Vec<T>, t: T) { v.push(t); let _l = v.len(); }
pub fn e16<T: Iterator>(mut t: T) -> Option<T::Item> { t.next() }
pub fn e17<T: Add<Output = T> + Copy>(a: T) -> T { let b = a; a + b }
pub trait Z: Sized { fn z(self) -> Self { self } fn zz(&self) -> Option<&Self> { Some(self) } }
pub fn e18<T: AsRef<[u8]>>(t: &T) -> usize { t.as_ref().len() }
pub fn e19<T: std::str::FromStr>(s: &str) -> Option<T> { s.parse().ok() }
pub fn e20<T: std::str::FromStr>(s: &str) -> Option<T> { s.parse::<T>().ok() }
pub fn e21<T>(o: Option<T>) -> T { match o { Some(x) => x, None => unreachable!() } }
pub fn e22<T: Default>() -> T { let x = Default::default(); x }
pub fn e23<'a, T: 'a>(v: &'a [T]) -> impl Iterator<Item = &'a T> { v.iter() }
pub fn e24<T: Clone>(v: &[T]) -> Vec<T> { v.to_vec() }
pub fn e25<T: Ord + Clone>(v: &mut Vec<T>) { v.sort(); v.dedup(); }
pub fn e26<T: Into<String>>(t: T) -> usize { let s: String = t.into(); s.len() }
pub fn e27<T>(t: Box<T>) -> T { *t }
pub fn e28<T: std::hash::Hash>(t: &T) -> u64 { use std::hash::Hasher; let mut h = std::collections::hash_map::DefaultHasher::new(); t.hash(&mut h); h.finish() }
pub fn e29<T: ToString + ?Sized>(t: &T) -> String { t.to_string() }
pub fn e30<T: Display>(t: T) -> String { format!("{t}") }
pub fn e31<T: Copy + Into<f64>>(a: T) -> f64 { a.into() * 2.0 }
pub fn e32<T: num::Zero>(t: T) -> bool { t.is_zero() }
pub mod num { pub trait Zero: Sized + std::ops::Add<Self, Output = Self> { fn zero() -> Self; fn is_zero(&self) -> bool; } }
pub fn e33<T: num::Zero + Copy>(a: T) -> T { a + T::zero() }
pub fn e34<T: Ord>(a: &T, b: &T) -> std::cmp::Ordering { a.cmp(b) }
pub fn e35<T: Ord>(a: T, b: T) -> bool { a < b }
pub fn e36<T: std::error::Error>(e: T) -> String { e.to_string() }
pub fn e37<T>(x: T) -> Option<T> { Some(x).map(|y| y) }
pub fn e38<T: Iterator<Item = T>>(t: T) -> Option<T> { t.into_iter().next() }
pub fn e39<T>(t: T) -> (T,) { (t,) }
pub fn e40<T: Clone>(t: &[T]) -> T { t[0].clone() }
pub fn e41<T: Clone>(t: &Vec<T>) -> T { t.first().unwrap().clone() }
pub fn e42<T, U: From<T>>(t: T) -> U { U::from(t) }
pub fn e43<T: Display>(t: Option<T>) -> String { t.map(|x| x.to_string()).unwrap_or_default() }
pub fn e44<T>(t: T) -> T where for<'a> &'a T: Add<&'a T, Output = T> { &t + &t }
pub fn e45<T: std::any::Any>(t: &T) -> bool { (t as &dyn std::any::Any).is::<i32>() }
pub fn e46<T: Default + std::ops::AddAssign + Copy>(v: &[T]) -> T { let mut s = T::default(); for x in v { s += *x; } s }
pub fn e47<T: std::ops::Index<usize>>(t: &T) -> &T::Output { &t[0] }
pub fn e48<T: Into<u8>>(t: T) -> bool { t.into() == 3 }
pub fn e49<T: PartialEq>(a: &[T], b: &[T]) -> bool { a == b }
pub fn e50<T: Clone>(a: &T) -> T { Clone::clone(a) }
pub fn e51<T: fmt::Debug>(a: T) { println!("{:?}", a); dbg!(&a); }
pub fn e52<T: Copy>(a: T) -> [T; 3] { [a; 3] }
pub fn e53<T>(a: Vec<T>) -> usize { a.iter().count() }
pub fn e54<T: Iterator>(t: T) -> Vec<T::Item> { t.collect() }
pub fn e55<T: Display>(t: &[T]) -> String { t.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",") }
pub fn e56<T: Clone + Into<u32>>(t: &T) -> u32 { t.clone().into() }
pub fn e57<T>(t: Option<&T>) -> bool { t.is_some() }
pub fn e58<T: std::ops::Not<Output = T>>(t: T) -> T { !t }
pub fn e59<S: AsRef<str>>(xs: &[S]) -> String { xs.iter().map(|s| s.as_ref()).collect() }
pub fn e60<T: Into<u64> + Copy>(t: T) -> u64 { let x: u64 = t.into(); x << 2 }
}
mod v3 {
use std::ops::Mul;
pub fn f<T: Into<u32> + Into<u64>>(t: T) -> u64 { t.into() }
pub fn g<T: Clone + Into<u32> + Into<u64>>(t: T) -> u64 { let a: u32 = t.clone().into(); let b: u64 = t.into(); b + a as u64 }
pub fn h<A, V, X: Mul<A, Output = A> + Mul<V, Output = V> + Copy>(x: X, a: A, v: V) -> (A, V) { (x * a, x * v) }
pub fn k<T: From<u32> + From<u8>>() -> T { T::from(3u8) }
pub fn m<T: PartialEq<u32> + PartialEq<u8>>(t: T) -> bool { t == 3u8 }
pub fn n<T: Mul<u32, Output = u32> + Mul<V, Output = V> + Copy, V>(x: T, v: V) -> V { x * v }
pub fn o<T: AsRef<str> + AsRef<[u8]>>(t: &T) -> usize { let s: &[u8] = t.as_ref(); s.len() }
pub fn p<T: Into<A> + Into<B>, A, B>(t: T) -> B { t.into() }
pub fn q<T: From<A> + From<B>, A, B>(b: B) -> T { T::from(b) }
pub fn r<T: From<A> + From<B>, A, B>(b: B) -> T { b.into() }
pub fn s<T: PartialEq<A> + PartialEq<B>, A, B>(t: T, b: B) -> bool { t == b }
pub trait Tr<X> { fn get(&self) -> X; }
pub fn u<T: Tr<u8> + Tr<u16>>(t: &T) -> u16 { t.get() }
pub fn v<T: Tr<u8> + Tr<u16>>(t: &T) -> u16 { Tr::<u16>::get(t) }
pub fn w<T: Tr<A> + Tr<B>, A, B>(t: &T) -> B { t.get() }
}
mod v4 {
use std::marker::PhantomData;
use std::fmt;
use std::collections::BTreeMap;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct P<T> { a: T, b: u32 }
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum E<T, U> { A(T), B { u: U }, C }
#[derive(Clone, Debug, Default)]
pub struct Ph<T> { p: PhantomData<T>, n: usize }
#[derive(Debug, Clone, PartialEq)]
pub struct Nest<T: Clone>(Vec<T>, Option<Box<Nest<T>>>);
pub struct Wrap<T>(T);
impl<T: fmt::Display> fmt::Display for Wrap<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "[{}]", self.0)?; writeln!(f)?; f.write_str("x") }
}
impl<T> Wrap<T> {
    fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Wrap<U> { Wrap(f(self.0)) }
    fn as_ref(&self) -> Wrap<&T> { Wrap(&self.0) }
    fn into_inner(self) -> T { self.0 }
}
impl<T: Iterator> Iterator for Wrap<T> { type Item = T::Item; fn next(&mut self) -> Option<T::Item> { self.0.next() } }
macro_rules! gen { ($n:ident) => { fn $n<T: Clone>(t: &T) -> (T, T) { (t.clone(), t.clone()) } } }
gen!(g1);
pub struct Stack<T> { v: Vec<T> }
impl<T> Stack<T> {
    fn new() -> Self { Stack { v: Vec::new() } }
    fn push(&mut self, t: T) -> &mut Self { self.v.push(t); self }
    fn pop(&mut self) -> Option<T> { self.v.pop() }
    fn peek(&self) -> Option<&T> { self.v.last() }
    fn len(&self) -> usize { self.v.len() }
    fn is_empty(&self) -> bool { self.len() == 0 }
    fn iter(&self) -> impl Iterator<Item = &T> + '_ { self.v.iter() }
    fn drain_all(&mut self) -> Vec<T> { std::mem::take(&mut self.v) }
}
impl<T: Ord + Clone> Stack<T> { fn max(&self) -> Option<T> { self.v.iter().max().cloned() } }
impl<T> Default for Stack<T> { fn default() -> Self { Self::new() } }
impl<T> FromIterator<T> for Stack<T> { fn from_iter<I: IntoIterator<Item = T>>(i: I) -> Self { Stack { v: i.into_iter().collect() } } }
impl<T> std::ops::Index<usize> for Stack<T> { type Output = T; fn index(&self, i: usize) -> &T { &self.v[i] } }
pub fn group<K: Ord + Clone, V>(xs: Vec<(K, V)>) -> BTreeMap<K, Vec<V>> { let mut m = BTreeMap::new(); for (k, v) in xs { m.entry(k).or_insert_with(Vec::new).push(v); } m }
pub trait Visitor { type Out; fn visit(&mut self, n: u32) -> Self::Out; fn visit_all(&mut self, ns: &[u32]) -> Vec<Self::Out> { ns.iter().map(|&n| self.visit(n)).collect() } }
pub trait Named { const NAME: &'static str; fn name(&self) -> &'static str { Self::NAME } fn upper(&self) -> String { Self::NAME.to_uppercase() } }
pub fn sorted<T: Ord>(mut v: Vec<T>) -> Vec<T> { v.sort_by(|a, b| a.cmp(b)); v }
pub fn pairs<T: Clone>(v: &[T]) -> Vec<(T, T)> { v.windows(2).map(|w| (w[0].clone(), w[1].clone())).collect() }
pub fn find<T: PartialEq>(v: &[T], x: &T) -> Option<usize> { v.iter().position(|y| y == x) }
pub fn tryf<T: std::str::FromStr>(s: &str) -> Result<T, T::Err> { let v = s.trim().parse::<T>()?; Ok(v) }
pub fn opt_chain<T: Clone>(o: Option<&T>) -> Option<T> { let x = o?; Some(x.clone()) }
pub fn err_conv<E: std::error::Error + 'static>(e: E) -> Box<dyn std::error::Error> { Box::new(e) }
pub fn rc<T>(t: T) -> std::rc::Rc<std::cell::RefCell<T>> { std::rc::Rc::new(std::cell::RefCell::new(t)) }
pub fn rcb<T: Clone>(r: &std::rc::Rc<std::cell::RefCell<T>>) -> T { r.borrow().clone() }
pub fn mtx<T: Default>(m: &std::sync::Mutex<T>) -> T { std::mem::take(&mut *m.lock().unwrap()) }
pub fn arr<T: Default + Copy, const N: usize>() -> [T; N] { [T::default(); N] }
pub fn slices<T: Copy>(v: &[T]) -> (T, &[T]) { match v { [first, rest @ ..] => (*first, rest), [] => panic!() } }
pub fn lets<T: Clone>(v: Option<T>) -> T { let Some(x) = v else { panic!() }; x }
pub fn cmp_ref<T: PartialOrd>(a: &T, b: &T) -> bool { *a < *b && a < b }
pub fn sum_it<T: std::iter::Sum<T> + Copy>(v: &[T]) -> T { v.iter().copied().sum() }
pub fn prod<T: std::iter::Product<T> + Copy>(v: &[T]) -> T { v.iter().copied().product() }
pub fn fold<T: Copy + std::ops::Add<Output = T> + Default>(v: &[T]) -> T { v.iter().fold(T::default(), |a, &b| a + b) }
}

fn main() {
    println!("{} {} {}", v1::sum(&[1, 2, 3]), v1::biggest(4, 2), v1::show(3));
    println!("{}", v1::cl(&String::from("x")));
    println!("{} {}", v2::e9(7u8, 3).len(), v2::e55(&[1, 2, 3]));
    println!("{:?}", v3::h(2u32, 3u32, 4u32));
    println!("{:?} {:?}", v4::sorted(vec![3, 1, 2]), v4::group(vec![(1, 'a'), (2, 'b'), (1, 'c')]));
}
