// #[macro_export], $crate, path-based macro invocation, macros used before definition via path
mod util {
    pub fn double(x: i32) -> i32 { x * 2 }
    pub struct Wrapper(pub i32);
    #[macro_export]
    macro_rules! dbl {
        ($e:expr) => { $crate::util::double($e) };
    }
    #[macro_export]
    macro_rules! wrap {
        ($e:expr) => { $crate::util::Wrapper($crate::dbl!($e)) };
    }
    macro_rules! private_mac { () => { "private" }; }
    pub(crate) use private_mac;
}

mod consumer {
    pub fn go() -> i32 { crate::dbl!(21) }
    pub fn go2() -> i32 { crate::wrap!(5).0 }
    pub fn go3() -> &'static str { crate::util::private_mac!() }
}

fn early() -> i32 { crate::late_mac!() + late_mac!() }

#[macro_export]
macro_rules! late_mac { () => { 100 }; }

mod reexp {
    macro_rules! hello { ($n:expr) => { format!("hello {}", $n) }; }
    pub(crate) use hello as greet;
}

use reexp::greet;

macro_rules! impl_ops {
    ($($t:ident),*) => {
        $(
            impl std::ops::Add for $t {
                type Output = $t;
                fn add(self, o: $t) -> $t { $t(self.0 + o.0) }
            }
            impl std::fmt::Display for $t {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    write!(f, "{}({})", stringify!($t), self.0)
                }
            }
        )*
    };
}

struct Meters(f64);
struct Feet(f64);
impl_ops!(Meters, Feet);

fn main() {
    println!("{}", dbl!(4));
    println!("{}", crate::dbl!(5));
    println!("{}", consumer::go());
    println!("{}", consumer::go2());
    println!("{}", consumer::go3());
    println!("{}", early());
    println!("{}", greet!("world"));
    println!("{}", reexp::greet!(1));
    println!("{} {}", Meters(1.5) + Meters(2.0), Feet(3.0) + Feet(0.25));
    println!("{}", self::late_mac!());
}
