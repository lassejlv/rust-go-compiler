//! A small math library.
pub mod shapes;

use std::ops::Add;

pub fn square(x: i32) -> i32 { x * x }

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec2 { pub x: f64, pub y: f64 }

impl Vec2 {
    pub fn new(x: f64, y: f64) -> Self { Vec2 { x, y } }
    pub fn len(&self) -> f64 { (self.x * self.x + self.y * self.y).sqrt() }
}

impl Add for Vec2 {
    type Output = Vec2;
    fn add(self, o: Vec2) -> Vec2 { Vec2::new(self.x + o.x, self.y + o.y) }
}

#[cfg(feature = "fancy")]
pub fn fancy_name() -> &'static str { "fancy enabled" }
#[cfg(not(feature = "fancy"))]
pub fn fancy_name() -> &'static str { "plain" }

#[doc(hidden)]
pub fn __max2(a: i32, b: i32) -> i32 { if a > b { a } else { b } }

#[macro_export]
macro_rules! max_of {
    ($x:expr) => { $x };
    ($x:expr, $($rest:expr),+) => { $crate::__max2($x, $crate::max_of!($($rest),+)) };
}
