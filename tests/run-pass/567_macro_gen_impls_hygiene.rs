// macros generating structs/impls/traits, hygiene of locals, macros defining macros, $crate paths, item fragments
use std::fmt;

trait Unit {
    const SYMBOL: &'static str;
    fn value(&self) -> f64;
}

macro_rules! units {
    ($($name:ident($sym:literal) => $factor:expr;)*) => {
        $(
            #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
            struct $name(f64);
            impl Unit for $name {
                const SYMBOL: &'static str = $sym;
                fn value(&self) -> f64 { self.0 }
            }
            impl fmt::Display for $name {
                fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    write!(f, "{}{}", self.0, <Self as Unit>::SYMBOL)
                }
            }
            impl $name {
                fn to_base(self) -> f64 { self.0 * $factor }
            }
            impl std::ops::Add for $name {
                type Output = $name;
                fn add(self, o: $name) -> $name { $name(self.0 + o.0) }
            }
        )*
    };
}

units! {
    Meters("m") => 1.0;
    Feet("ft") => 0.3048;
    Miles("mi") => 1609.344;
}

macro_rules! swap_vars {
    ($a:ident, $b:ident) => {
        let tmp = $a;
        $a = $b;
        $b = tmp;
    };
}

macro_rules! with_temp {
    ($e:expr) => {{
        let x = 1000;
        $e + x
    }};
}

macro_rules! make_adder_macro {
    ($name:ident, $n:expr) => {
        macro_rules! $name {
            ($v:expr) => {
                $v + $n
            };
        }
    };
}
make_adder_macro!(add10, 10);
make_adder_macro!(add_neg, -1);

macro_rules! getters {
    ($t:ident { $($field:ident : $ty:ty),* }) => {
        struct $t { $($field: $ty),* }
        impl $t {
            $(
                fn $field(&self) -> &$ty { &self.$field }
            )*
        }
    };
}
getters!(Config { name: String, retries: u32, ratio: f32 });

macro_rules! impl_trait_for {
    ($tr:ident => $($t:ty),+) => {
        $(impl $tr for $t {
            fn describe(&self) -> String { format!("{}({:?})", stringify!($t), self) }
        })+
    };
}
trait Describe {
    fn describe(&self) -> String;
}
impl_trait_for!(Describe => i32, &str, Vec<u8>, (bool, char));

macro_rules! make_fn {
    ($vis:vis fn $name:ident() -> $ret:ty $body:block) => {
        $vis fn $name() -> $ret $body
    };
}
make_fn!(pub fn forty_two() -> i32 { 42 });
make_fn!(fn greeting() -> String { "hi".repeat(2) });

macro_rules! test_cases {
    ($($name:ident: $input:expr => $expected:expr,)*) => {
        fn run_cases() -> Vec<(&'static str, bool)> {
            vec![$((stringify!($name), $input == $expected)),*]
        }
    };
}
test_cases! {
    adds: 1 + 1 => 2,
    strings: "a".to_uppercase() => "A",
    fails: 2 * 2 => 5,
}

macro_rules! log_expr {
    ($($e:expr),+ $(,)?) => {{
        let mut out = Vec::new();
        $( out.push(format!("{} = {:?}", stringify!($e), $e)); )+
        out
    }};
}

fn main() {
    let a = Meters(5.0) + Meters(2.5);
    let b = Feet(10.0);
    let c = Miles(0.5);
    println!("{} {} {}", a, b, c);
    println!("{:.3} {:.3} {:.3}", a.to_base(), b.to_base(), c.to_base());
    println!("{} {} {}", Meters::SYMBOL, Feet::SYMBOL, <Miles as Unit>::SYMBOL);
    println!("{}", Feet(3.0) < Feet(4.0));
    let mut x = 1;
    let mut y = 2;
    swap_vars!(x, y);
    println!("{} {}", x, y);
    let tmp = "outer tmp";
    swap_vars!(x, y);
    println!("{} {} {}", x, y, tmp);
    let x = 5;
    println!("{}", with_temp!(x * 2));
    println!("{} {}", add10!(5), add_neg!(add10!(0)));
    let cfg = Config { name: "svc".into(), retries: 3, ratio: 0.5 };
    println!("{} {} {}", cfg.name(), cfg.retries(), cfg.ratio());
    println!("{}", 7.describe());
    println!("{}", "s".describe());
    println!("{}", vec![1u8, 2].describe());
    println!("{}", (true, 'x').describe());
    println!("{} {}", forty_two(), greeting());
    println!("{:?}", run_cases());
    let v = vec![1, 2, 3];
    for l in log_expr!(v.len(), v.iter().sum::<i32>(), x + 1, "lit",) {
        println!("{}", l);
    }
    println!("{}", line!() > 0);
    println!("{}", concat!("a", 1, 2.5, true, 'c'));
    println!("{}", stringify!(a + b * (c - d)));
    println!("{:?}", module_path!());
}
