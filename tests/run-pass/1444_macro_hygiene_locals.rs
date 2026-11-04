// hygiene: locals in macros are not visible outside; items are
macro_rules! make_local {
    ($v:expr) => { let x = $v; };
}

macro_rules! make_named {
    ($name:ident, $v:expr) => { let $name = $v; };
}


macro_rules! define_fn {
    () => { fn generated() -> &'static str { "gen" } };
}
define_fn!();

macro_rules! shadow {
    ($e:expr) => {{ let a = 100; $e + a }};
}

macro_rules! swap {
    ($a:ident, $b:ident) => { let tmp = $a; $a = $b; $b = tmp; };
}

macro_rules! loop_label {
    ($e:expr) => { 'outer: loop { if $e { break 'outer 1; } break 'outer 2; } };
}

macro_rules! make_struct_and_impl {
    ($n:ident) => {
        struct $n;
        impl $n { fn hi(&self) -> String { format!("hi from {}", stringify!($n)) } }
    };
}

fn main() {
    let x = 1;
    make_local!(99);
    println!("{}", x);
    make_named!(y, 5);
    println!("{}", y);
    let x = 3;
    println!("{}", generated());
    let a = 1;
    println!("{}", shadow!(a));
    let mut p = 1;
    let mut q = 2;
    let tmp = 77;
    swap!(p, q);
    println!("{} {} {}", p, q, tmp);
    println!("{}", loop_label!(x > 2));
    make_struct_and_impl!(Thing);
    println!("{}", Thing.hi());
    macro_rules! local_mac { ($e:expr) => { $e * x }; }
    println!("{}", local_mac!(4));
    {
        let x = 1000;
        println!("{}", local_mac!(1) + x);
    }
}
