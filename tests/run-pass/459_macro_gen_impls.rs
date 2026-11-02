// Macros generating trait impls for many types, enums with Display/FromStr generated from a table,
// newtype wrappers with operator impls, and a macro that builds a test table of (input, expected).
use std::fmt;
use std::ops::{Add, Sub};
use std::str::FromStr;

trait Describe {
    fn describe() -> String;
    fn bits(&self) -> u32;
}

macro_rules! impl_describe {
    ($($t:ty => $signed:expr),+ $(,)?) => {
        $(
            impl Describe for $t {
                fn describe() -> String {
                    format!("{}: {} bytes, signed={}, min={}, max={}", stringify!($t), std::mem::size_of::<$t>(), $signed, <$t>::MIN, <$t>::MAX)
                }
                fn bits(&self) -> u32 {
                    self.count_ones()
                }
            }
        )+
    };
}
impl_describe!(u8 => false, i8 => true, u16 => false, i32 => true, u64 => false, i128 => true);

macro_rules! string_enum {
    ($name:ident { $($variant:ident = $text:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq)]
        enum $name { $($variant),+ }
        impl $name {
            const ALL: &'static [$name] = &[$($name::$variant),+];
        }
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                let s = match self { $($name::$variant => $text),+ };
                f.pad(s)
            }
        }
        impl FromStr for $name {
            type Err = String;
            fn from_str(s: &str) -> Result<Self, String> {
                match s {
                    $($text => Ok($name::$variant),)+
                    other => Err(format!("unknown {} {:?}", stringify!($name), other)),
                }
            }
        }
    };
}
string_enum!(Method { Get = "GET", Post = "POST", Delete = "DELETE" });
string_enum!(Color { Red = "red", Green = "green" });

macro_rules! newtype {
    ($name:ident($inner:ty), $unit:literal) => {
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
        struct $name($inner);
        impl Add for $name {
            type Output = $name;
            fn add(self, o: $name) -> $name {
                $name(self.0 + o.0)
            }
        }
        impl Sub for $name {
            type Output = $name;
            fn sub(self, o: $name) -> $name {
                $name(self.0 - o.0)
            }
        }
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "{}{}", self.0, $unit)
            }
        }
        impl From<$inner> for $name {
            fn from(v: $inner) -> $name {
                $name(v)
            }
        }
    };
}
newtype!(Seconds(u64), "s");
newtype!(Celsius(f64), "°C");

macro_rules! table_test {
    ($f:expr; $($input:expr => $expected:expr),* $(,)?) => {{
        let mut pass = 0;
        let mut fail = Vec::new();
        $(
            let got = $f($input);
            if got == $expected { pass += 1; } else { fail.push(format!("{:?}: got {:?} want {:?}", $input, got, $expected)); }
        )*
        (pass, fail)
    }};
}

macro_rules! getters {
    ($s:ident { $($f:ident : $t:ty),* }) => {
        struct $s { $($f: $t),* }
        impl $s {
            $( fn $f(&self) -> &$t { &self.$f } )*
        }
    };
}
getters!(Person { name: String, age: u8 });

fn main() {
    println!("{}", u8::describe());
    println!("{}", i8::describe());
    println!("{}", u16::describe());
    println!("{}", i32::describe());
    println!("{}", u64::describe());
    println!("{}", i128::describe());
    println!("{} {} {}", 255u8.bits(), (-1i32).bits(), (1i128 << 100).bits());

    for m in Method::ALL {
        println!("[{:>8}] {:?}", m, m);
    }
    println!("{:?} {:?}", "POST".parse::<Method>(), "PUT".parse::<Method>());
    println!("{:?} {:?}", "green".parse::<Color>(), Color::ALL.len());

    let t = Seconds(30) + Seconds::from(45) - Seconds(5);
    println!("{} {:?} {}", t, Seconds::default(), t > Seconds(60));
    let c = Celsius(21.5) + Celsius(-1.25);
    println!("{} {}", c, Celsius::from(0.0) < c);

    let (pass, fail) = table_test!(|s: &str| s.len(); "a" => 1, "abc" => 3, "" => 1, "héllo" => 5);
    println!("pass={} fail={:?}", pass, fail);
    let (pass, fail) = table_test!(|n: u32| n.is_power_of_two(); 1 => true, 6 => false, 64 => true);
    println!("pass={} fail={:?}", pass, fail);

    let p = Person { name: "Ada".into(), age: 36 };
    println!("{} {}", p.name(), p.age());
}
