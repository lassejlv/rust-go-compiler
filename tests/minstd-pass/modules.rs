mod geometry {
    pub mod shapes {
        pub struct Sq(pub i32);
        impl Sq { pub fn area(&self) -> i32 { self.0 * self.0 } }
        pub fn unit() -> Sq { Sq(1) }
    }
    pub use self::shapes::Sq;
    pub fn double(s: &Sq) -> i32 { super::helper(s.area()) }
}
use geometry::shapes::{self, unit};
fn helper(x: i32) -> i32 { x * 2 }
const LIMIT: i32 = 3 * 4;
static mut COUNTER: i32 = 0;
static GREETING: &str = "hi";
macro_rules! square { ($e:expr) => { $e * $e }; }
macro_rules! sum { () => { 0 }; ($h:expr $(, $t:expr)*) => { $h + sum!($($t),*) }; }
struct Buf<const N: usize> { data: [i32; N] }
impl<const N: usize> Buf<N> { fn cap(&self) -> usize { N } fn total(&self) -> i32 { let mut s = 0; let mut i = 0; while i < N { s += self.data[i]; i += 1; } s } }
fn main() {
    let s = geometry::Sq(5);
    println!("{} {}", s.area(), geometry::double(&s));
    println!("{}", unit().area() + shapes::unit().area());
    println!("limit {}", LIMIT);
    unsafe { COUNTER += 5; COUNTER += 1; println!("counter {}", COUNTER); }
    println!("{}", GREETING);
    println!("sq {}", square!(3 + 1));
    println!("sum {}", sum!(1, 2, 3, 4));
    fn inner(x: i32) -> i32 { x + 100 }
    println!("inner {}", inner(1));
    let b = Buf { data: [1, 2, 3, 4] };
    println!("cap {} total {}", b.cap() as i32, b.total());
    let arr = [[1, 2], [3, 4]];
    println!("arr {}", arr[1][0]);
    let x = 5; let x = x * 2; { let x = 1; println!("shadow {}", x); } println!("x {}", x);
    let t = if LIMIT > 10 { "big" } else { "small" };
    println!("{}", t);
}
