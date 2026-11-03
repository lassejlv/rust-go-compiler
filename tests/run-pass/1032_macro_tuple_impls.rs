trait Arity { const N: usize; fn names(&self) -> Vec<&'static str>; }
trait SumAll { fn sum_all(&self) -> i64; }

macro_rules! peel {
    ($first:ident $($rest:ident)*) => {
        impl_tuple!($first $($rest)*);
        peel!($($rest)*);
    };
    () => {};
}

macro_rules! impl_tuple {
    ($($t:ident)+) => {
        impl<$($t: Into<i64> + Copy),+> SumAll for ($($t,)+) {
            #[allow(non_snake_case)]
            fn sum_all(&self) -> i64 {
                let ($($t,)+) = *self;
                0 $(+ $t.into())+
            }
        }
        impl<$($t),+> Arity for ($($t,)+) {
            const N: usize = [$(stringify!($t)),+].len();
            fn names(&self) -> Vec<&'static str> { vec![$(stringify!($t)),+] }
        }
    };
}

peel!(A B C D E F);

macro_rules! max_of {
    ($x:expr) => { $x };
    ($x:expr, $($rest:expr),+) => {{ let a = $x; let b = max_of!($($rest),+); if a > b { a } else { b } }};
}

macro_rules! try_all {
    ($($e:expr),* $(,)?) => {{
        (|| -> Result<Vec<i32>, std::num::ParseIntError> {
            Ok(vec![$($e.parse::<i32>()?),*])
        })()
    }};
}

macro_rules! delims {
    ($($t:tt)*) => { concat!("[", stringify!($($t)*), "]") };
}

struct Calc;
macro_rules! methods {
    ($($name:ident => $op:tt),*) => {
        impl Calc { $(fn $name(a: i32, b: i32) -> i32 { a $op b })* }
    };
}
methods!(add => +, sub => -, shl => <<, bitand => &);

trait Greeter { fn greet(&self) -> String; }
macro_rules! greeter_impls {
    ($($t:ty => $msg:literal;)*) => { $(impl Greeter for $t { fn greet(&self) -> String { format!("{} says {}", stringify!($t), $msg) } })* };
}
greeter_impls! { u8 => "byte"; bool => "flag"; &str => "text"; }

macro_rules! let_destructure {
    ($p:pat = $e:expr => $body:expr) => {{ let $p = $e; $body }};
}

fn main() {
    println!("{}", (1u8,).sum_all());
    println!("{}", (1u8, 2i16, 3i32).sum_all());
    println!("{}", (1u8, 2u16, 3u32, 4i8, 5i16, 6i32).sum_all());
    println!("{} {:?}", <(u8, u8, u8, u8) as Arity>::N, ((), (), ()).names());
    println!("{}", max_of!(3, 9, 2, 7));
    println!("{}", max_of!("pear", "apple", "zoo"));
    println!("{:?}", try_all!("1", "22", "333"));
    println!("{:?}", try_all!("1", "x").is_err());
    println!("{}", delims!(a [b] { c }));
    println!("{}", delims![x y]);
    println!("{}", delims! { q });
    println!("{} {} {} {}", Calc::add(2, 3), Calc::sub(2, 3), Calc::shl(1, 4), Calc::bitand(6, 3));
    println!("{} | {} | {}", 7u8.greet(), true.greet(), "hi".greet());
    println!("{}", let_destructure!((a, [b, c]) = (1, [2, 3]) => a * 100 + b * 10 + c));
}
