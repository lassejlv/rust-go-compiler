// macros generating modules, use-declarations, consts and tests-like registries
macro_rules! units {
    ($($m:ident => $factor:expr),* $(,)?) => {
        $(
            pub mod $m {
                #[allow(unused_imports)]
                use super::*;
                pub const FACTOR: f64 = $factor;
                pub fn to_base(v: f64) -> f64 { v * FACTOR }
                pub fn name() -> &'static str { stringify!($m) }
                pub fn path() -> &'static str { module_path!() }
            }
        )*
        pub fn all_units() -> Vec<(&'static str, f64)> { vec![$(($m::name(), $m::FACTOR)),*] }
    };
}

pub fn helper() -> i32 { 5 }

units!(mm => 0.001, cm => 0.01, km => 1000.0);

macro_rules! registry {
    ($reg:ident: $($name:ident = $body:block)*) => {
        $( fn $name() -> i32 $body )*
        static $reg: &[(&str, fn() -> i32)] = &[$((stringify!($name), $name as fn() -> i32)),*];
    };
}
registry!(TESTS:
    one = { 1 }
    two = { helper() * 2 }
    three = { let x = 3; x }
);

macro_rules! reexport {
    ($($p:ident)::+ as $alias:ident) => { use $($p)::+ as $alias; };
}
reexport!(std::collections::BTreeMap as Map);

fn main() {
    println!("{} {} {}", mm::to_base(5.0), cm::path(), km::name());
    println!("{:?}", all_units());
    for (n, f) in TESTS { println!("{} => {}", n, f()); }
    let mut m: Map<i32, &str> = Map::new();
    m.insert(2, "b"); m.insert(1, "a");
    println!("{:?}", m);
}
