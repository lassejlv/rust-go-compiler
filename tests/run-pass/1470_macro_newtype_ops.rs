// newtype + ops-forwarding macro family with generics and lifetimes
use std::fmt;
use std::ops::{Add, Mul, Sub, Neg, AddAssign, Index};

macro_rules! newtype {
    ($(#[$attr:meta])* $vis:vis struct $name:ident($inner:ty); $($op:ident :: $method:ident),* $(,)?) => {
        $(#[$attr])*
        $vis struct $name(pub $inner);
        $( impl $op for $name {
            type Output = $name;
            fn $method(self, rhs: $name) -> $name { $name(self.0.$method(rhs.0)) }
        } )*
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, concat!(stringify!($name), "<{}>"), self.0)
            }
        }
        impl From<$inner> for $name { fn from(v: $inner) -> Self { $name(v) } }
    };
}

newtype!(#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)] pub struct Meters(f64); Add::add, Sub::sub, Mul::mul);
newtype!(#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)] struct Id(u32); Add::add);

macro_rules! impl_neg_assign {
    ($t:ident) => {
        impl Neg for $t { type Output = $t; fn neg(self) -> $t { $t(-self.0) } }
        impl AddAssign for $t { fn add_assign(&mut self, o: $t) { self.0 += o.0; } }
    };
}
impl_neg_assign!(Meters);

macro_rules! view {
    ($name:ident<$lt:lifetime, $t:ident>) => {
        struct $name<$lt, $t> { items: &$lt [$t] }
        impl<$lt, $t> Index<usize> for $name<$lt, $t> {
            type Output = $t;
            fn index(&self, i: usize) -> &$t { &self.items[self.items.len() - 1 - i] }
        }
        impl<$lt, $t: fmt::Debug> fmt::Debug for $name<$lt, $t> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_list().entries(self.items.iter().rev()).finish()
            }
        }
    };
}
view!(Rev<'a, T>);

fn main() {
    let a = Meters(1.5) + Meters(2.0) * Meters(2.0);
    println!("{} {:?}", a, a);
    let mut b = -a;
    b += Meters::from(10.0);
    println!("{} {}", b, b > a);
    let i = Id(1) + Id::from(41);
    println!("{} {:?} {}", i, i, i == Id(42));
    let data = [1, 2, 3];
    let r = Rev { items: &data };
    println!("{} {:?}", r[0], r);
}
