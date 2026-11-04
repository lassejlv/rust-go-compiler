// macro-defining macros via $ passed as tt, shadowing, textual scope, #[macro_use] mod
macro_rules! make_adder {
    ($d:tt $name:ident, $n:expr) => {
        macro_rules! $name {
            ($d x:expr) => { $d x + $n };
            ($d x:expr, $d ($d rest:expr),+) => { $name!($d x) $d (+ $name!($d rest))+ };
        }
    };
}
make_adder!($ add10, 10);
make_adder!($ add1, 1);

macro_rules! gen_getters {
    ($d:tt $($f:ident),*) => {
        macro_rules! getter_names { () => { [$(stringify!($f)),*] }; }
    };
}
gen_getters!($ a, b, c);

#[macro_use]
mod macros {
    macro_rules! from_mod { () => { "from mod" }; }
    pub fn helper() -> &'static str { from_mod!() }
}

mod user {
    pub fn use_it() -> &'static str { from_mod!() }
}

macro_rules! shadowed { () => { 1 }; }

fn first() -> i32 { shadowed!() }

macro_rules! shadowed { () => { 2 }; }

fn second() -> i32 { shadowed!() }

macro_rules! recursive_def {
    () => {};
    ($head:ident $($tail:ident)*) => {
        fn $head() -> &'static str { stringify!($head) }
        recursive_def!($($tail)*);
    };
}
recursive_def!(fa fb fc);

fn main() {
    println!("{} {}", add10!(5), add1!(1, 2, 3));
    println!("{:?}", getter_names!());
    println!("{} {}", macros::helper(), user::use_it());
    println!("{} {}", first(), second());
    println!("{}", shadowed!());
    macro_rules! shadowed { () => { 3 }; }
    println!("{}", shadowed!());
    {
        macro_rules! shadowed { () => { 4 }; }
        println!("{}", shadowed!());
    }
    println!("{}", shadowed!());
    println!("{} {} {}", fa(), fb(), fc());
}
