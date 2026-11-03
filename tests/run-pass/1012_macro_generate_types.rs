use std::fmt;

macro_rules! make_struct {
    ($(#[$m:meta])* $vis:vis struct $name:ident { $($fvis:vis $field:ident : $ty:ty = $default:expr),* $(,)? }) => {
        $(#[$m])*
        $vis struct $name { $($fvis $field: $ty),* }
        impl Default for $name {
            fn default() -> Self { $name { $($field: $default),* } }
        }
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{} {{", stringify!($name))?;
                let mut first = true;
                $(
                    if !first { write!(f, ",")?; }
                    first = false;
                    write!(f, " {}: {}", stringify!($field), self.$field)?;
                )*
                let _ = first;
                write!(f, " }}")
            }
        }
        impl $name {
            pub fn field_names() -> &'static [&'static str] { &[$(stringify!($field)),*] }
        }
    };
}

make_struct! {
    #[derive(Debug, Clone, PartialEq)]
    pub struct Config {
        pub name: String = "default".to_string(),
        retries: u32 = 3,
        pub(crate) ratio: f64 = 0.5,
    }
}

macro_rules! str_enum {
    ($name:ident { $($var:ident => $s:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        enum $name { $($var),+ }
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str(match self { $(Self::$var => $s),+ })
            }
        }
        impl std::str::FromStr for $name {
            type Err = String;
            fn from_str(s: &str) -> Result<Self, String> {
                match s { $($s => Ok(Self::$var),)+ other => Err(format!("bad {}: {}", stringify!($name), other)) }
            }
        }
    };
}

str_enum!(Method { Get => "GET", Post => "POST", Delete => "DELETE" });

macro_rules! impl_ops {
    ($t:ident : $($tr:ident :: $m:ident => $op:tt),*) => {
        $(
            impl std::ops::$tr for $t {
                type Output = $t;
                fn $m(self, o: $t) -> $t { $t(self.0 $op o.0) }
            }
        )*
    };
}

#[derive(Debug, Clone, Copy)]
struct W(i64);
impl_ops!(W: Add::add => +, Sub::sub => -, Mul::mul => *, Rem::rem => %);

macro_rules! newtypes {
    ($($name:ident($inner:ty)),*) => {
        $(
            #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
            struct $name($inner);
            impl From<$inner> for $name { fn from(v: $inner) -> Self { $name(v) } }
        )*
    };
}
newtypes!(Meters(f64), Count(u32), Flag(bool));

fn main() {
    let c = Config::default();
    println!("{}", c);
    println!("{:?}", c);
    println!("{:?}", Config::field_names());
    let c2 = Config { retries: 9, ..c.clone() };
    println!("{} eq={}", c2, c2 == c);
    for m in [Method::Delete, Method::Get, Method::Post] {
        print!("{} ", m);
    }
    println!();
    println!("{:?} {:?}", "POST".parse::<Method>(), "PUT".parse::<Method>());
    let mut ms = vec![Method::Post, Method::Delete, Method::Get];
    ms.sort();
    println!("{:?}", ms);
    let r = (W(10) + W(5)) * W(3) - W(4) % W(3);
    println!("{:?}", r);
    let m: Meters = 2.5.into();
    println!("{:?} {:?} {:?}", m, Count::from(3), Flag(true));
}
