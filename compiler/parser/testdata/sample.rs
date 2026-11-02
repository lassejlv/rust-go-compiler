#!/usr/bin/env run-cargo-script
//! A sample program exercising lots of Rust syntax.
#![allow(dead_code, unused_variables)]

use std::collections::{HashMap, HashSet};
use std::fmt::{self, Display, Formatter};
use std::ops::{Add, Index as Idx};
use std::rc::Rc;
use ::std::cell::RefCell;

mod geometry {
    pub mod shapes {
        #[derive(Debug, Clone, Copy, PartialEq, Default)]
        pub struct Point {
            pub x: f64,
            pub y: f64,
        }

        impl Point {
            pub const ORIGIN: Point = Point { x: 0.0, y: 0.0 };

            pub fn new(x: f64, y: f64) -> Self {
                Self { x, y }
            }

            pub fn dist(&self, other: &Point) -> f64 {
                ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
            }
        }
    }

    pub(crate) fn area(w: f64, h: f64) -> f64 {
        w * h
    }
}

use geometry::shapes::Point;

/// A shape trait with an associated type and default method.
pub trait Shape: fmt::Debug {
    type Unit: Copy + Default;
    const SIDES: u32 = 0;

    fn area(&self) -> f64;
    fn name(&self) -> String {
        String::from("shape")
    }
    fn scaled(self: Box<Self>, k: f64) -> Box<dyn Shape<Unit = Self::Unit>>
    where
        Self: Sized + 'static;
}

#[derive(Debug)]
enum Expr {
    Num(i64),
    Add(Box<Expr>, Box<Expr>),
    Mul { lhs: Box<Expr>, rhs: Box<Expr> },
    Neg(Box<Expr>),
    Var(String),
    Unit,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Color {
    Red = 1,
    Green = 2,
    Blue = 4,
}

impl Expr {
    fn eval(&self, env: &HashMap<String, i64>) -> Result<i64, String> {
        Ok(match self {
            Expr::Num(n) => *n,
            Expr::Add(a, b) => a.eval(env)? + b.eval(env)?,
            Expr::Mul { lhs, rhs } => lhs.eval(env)? * rhs.eval(env)?,
            Expr::Neg(e) => -e.eval(env)?,
            Expr::Var(name) => match env.get(name) {
                Some(v) => *v,
                None => return Err(format!("unbound variable {}", name)),
            },
            Expr::Unit => 0,
        })
    }
}

struct Matrix<T, const N: usize> {
    data: [[T; N]; N],
}

impl<T: Copy + Default + Add<Output = T>, const N: usize> Matrix<T, N> {
    fn identity_like(v: T) -> Self {
        let mut data = [[T::default(); N]; N];
        for i in 0..N {
            data[i][i] = v;
        }
        Matrix { data }
    }

    fn trace(&self) -> T {
        let mut acc = T::default();
        let mut i = 0;
        while i < N {
            acc = acc + self.data[i][i];
            i += 1;
        }
        acc
    }
}

impl<T, const N: usize> Idx<(usize, usize)> for Matrix<T, N> {
    type Output = T;
    fn index(&self, (r, c): (usize, usize)) -> &T {
        &self.data[r][c]
    }
}

struct Wrapper<'a, T: ?Sized + 'a>(&'a T);

impl<'a, T> Display for Wrapper<'a, T>
where
    T: Display + ?Sized,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "[{}]", self.0)
    }
}

union IntOrFloat {
    i: u32,
    f: f32,
}

static mut COUNTER: u32 = 0;
const LIMIT: usize = 1 << 4;
type Callback<'a> = Box<dyn Fn(i32) -> i32 + 'a>;

macro_rules! square {
    ($x:expr) => {
        $x * $x
    };
    ($x:expr, $($rest:expr),+) => {
        square!($x) + square!($($rest),+)
    };
}

extern "C" {
    fn abs(x: i32) -> i32;
    fn printf(fmt: *const u8, ...) -> i32;
}

unsafe fn bump() -> u32 {
    COUNTER += 1;
    COUNTER
}

const fn cube(x: u64) -> u64 {
    x * x * x
}

fn apply<F>(f: F, v: i32) -> i32
where
    F: Fn(i32) -> i32,
{
    f(v)
}

fn make_adder(n: i32) -> impl Fn(i32) -> i32 {
    move |x| x + n
}

fn compose<'a, A, B, C>(f: &'a dyn Fn(A) -> B, g: &'a dyn Fn(B) -> C) -> Callback<'a>
where
    A: From<i32>,
    C: Into<i32>,
    for<'b> &'b B: Sized,
{
    Box::new(move |x| g(f(A::from(x))).into())
}

fn classify(n: i32) -> &'static str {
    match n {
        i32::MIN..=-1 => "negative",
        0 => "zero",
        1 | 2 | 3 => "small",
        x if x % 2 == 0 => "even",
        _ => "odd",
    }
}

fn slices(v: &[i32]) -> Option<(i32, i32)> {
    match v {
        [] => None,
        [one] => Some((*one, *one)),
        [first, .., last] => Some((*first, *last)),
    }
}

fn parse_kv(s: &str) -> Option<(&str, i32)> {
    let (k, v) = s.split_once('=')?;
    let Ok(n) = v.trim().parse::<i32>() else {
        return None;
    };
    Some((k.trim(), n))
}

fn labeled() -> i32 {
    let mut total = 0;
    'outer: for i in 0..10 {
        let mut j = 0;
        loop {
            if j > i {
                continue 'outer;
            }
            if i * j > 20 {
                break 'outer;
            }
            total += j;
            j += 1;
        }
    }
    let v = 'blk: {
        if total > 100 {
            break 'blk 1;
        }
        2
    };
    total + v
}

fn tuples() -> (i32, f64) {
    let t = ((1, 2), (3.5, 4));
    let (a, b): (i32, i32) = t.0;
    let x = t.1.0;
    let s = t.0.1;
    (a + b + s, x)
}

fn closures() -> Vec<i32> {
    let v: Vec<i32> = (1..=10).filter(|&x| x % 2 == 1).map(|x| x * x).collect();
    let pairs: Vec<(i32, i32)> = v.iter().enumerate().map(|(i, &x)| (i as i32, x)).collect();
    let sum: i32 = pairs.iter().map(|&(a, b)| a + b).sum();
    let f = || -> i32 { 42 };
    let g = |a: i32, b: i32| -> i32 { a.max(b) };
    let mut out = Vec::<i32>::with_capacity(LIMIT);
    out.push(sum + f() + g(1, 2) + 5.max(3) + 1.0f64.sqrt() as i32);
    out
}

fn shared() {
    let shared = Rc::new(RefCell::new(vec![1, 2, 3]));
    let other = Rc::clone(&shared);
    other.borrow_mut().push(4);
    if let Some(last) = shared.borrow().last() && *last > 3 {
        println!("last = {last}");
    }
    let set: HashSet<_> = shared.borrow().iter().cloned().collect();
    let n = <Vec<i32>>::new().len() + <i32 as Default>::default() as usize;
    let _ = (set, n);
}

fn main() {
    let p = Point::new(3.0, 4.0);
    let q = Point { x: 1.0, ..Point::ORIGIN };
    println!("{:?} {:.2}", p, p.dist(&q));
    let mut env = HashMap::new();
    env.insert("x".to_string(), 5);
    let e = Expr::Add(Box::new(Expr::Num(2)), Box::new(Expr::Mul {
        lhs: Box::new(Expr::Var("x".into())),
        rhs: Box::new(Expr::Neg(Box::new(Expr::Num(3)))),
    }));
    match e.eval(&env) {
        Ok(v) => println!("value = {}", v),
        Err(msg) => eprintln!("error: {}", msg),
    }
    let m: Matrix<i64, 3> = Matrix::identity_like(7);
    assert_eq!(m.trace(), 21);
    assert_eq!(m[(1, 1)], 7);
    let w = Wrapper("hi");
    let s = format!("{}", w) + "!";
    let c = Color::Green as u8 | Color::Blue as u8;
    let u = IntOrFloat { i: 0x3f80_0000 };
    let f = unsafe { u.f };
    let k = unsafe { bump() } + unsafe { abs(-3) } as u32;
    let adder = make_adder(10);
    let r = apply(&adder, square!(2, 3));
    let x = if r > 10 { r - 10 } else if r < 0 { -r } else { 0 };
    let raw = r#"raw "string""#;
    let bytes = b"bytes\n";
    let ch = '\u{1F600}';
    let big = 340282366920938463463374607431768211455u128;
    let mut arr = [0u8; 4];
    arr[0] = b'a';
    for (i, item) in arr.iter_mut().enumerate().skip(1) {
        *item = i as u8 * 2;
    }
    let tail = vec![1, 2, 3].into_iter().rev().collect::<Vec<_>>();
    let neg = -x.pow(2);
    let cond = !tuples().0.is_positive() && classify(x) != "odd" || labeled() >= 3;
    let range = ..=5;
    let _all = (p, q, s, c, f, k, raw, bytes, ch, big, arr, tail, neg, cond, range, closures(), slices(&[1, 2]), parse_kv("a=1"), cube(3), geometry::area(1.0, 2.0));
    shared();
}
