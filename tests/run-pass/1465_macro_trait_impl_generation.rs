// macros generating trait impls for many types, generics with bounds, assoc types
use std::fmt::Debug;

trait Zero { const ZERO: Self; fn is_zero(&self) -> bool; }
macro_rules! impl_zero {
    ($($t:ty => $z:expr),+ $(,)?) => {
        $( impl Zero for $t {
            const ZERO: Self = $z;
            fn is_zero(&self) -> bool { *self == $z }
        } )+
    };
}
impl_zero!(i32 => 0, u64 => 0, f64 => 0.0, char => '\0', bool => false);

trait Named { fn name() -> String; }
macro_rules! impl_named_generic {
    ($($outer:ident < $($p:ident),+ >),*) => {
        $( impl<$($p: Named),+> Named for $outer<$($p),+> {
            fn name() -> String {
                let parts: Vec<String> = vec![$($p::name()),+];
                format!("{}<{}>", stringify!($outer), parts.join(", "))
            }
        } )*
    };
}
macro_rules! impl_named_leaf { ($($t:ident),*) => { $( impl Named for $t { fn name() -> String { stringify!($t).to_string() } } )* }; }
impl_named_leaf!(i32, u8, String);
use std::collections::HashMap;
impl_named_generic!(Vec<T>, Option<T>, HashMap<K, V>);

macro_rules! tuple_impls {
    ($( ($($T:ident $i:tt),+) )+) => {
        $( impl<$($T: Named),+> Named for ($($T,)+) {
            fn name() -> String {
                let v: Vec<String> = vec![$($T::name()),+];
                format!("({})", v.join(", "))
            }
        } )+
    };
}
tuple_impls! { (A 0) (A 0, B 1) (A 0, B 1, C 2) }

trait Container { type Item; fn first(&self) -> Option<&Self::Item>; }
macro_rules! impl_container {
    ($name:ident<$t:ident> => $field:ident) => {
        struct $name<$t> { $field: Vec<$t> }
        impl<$t: Debug> Container for $name<$t> {
            type Item = $t;
            fn first(&self) -> Option<&$t> { self.$field.first() }
        }
    };
}
impl_container!(Stack<T> => items);

macro_rules! sum_tuple {
    ($t:expr; $($i:tt)+) => { 0 $(+ $t.$i)+ };
}

fn main() {
    println!("{} {} {} {}", 0i32.is_zero(), 1u64.is_zero(), f64::ZERO, char::ZERO as u32);
    println!("{}", <bool as Zero>::ZERO);
    println!("{}", <Vec<Option<i32>>>::name());
    println!("{}", <HashMap<String, Vec<u8>>>::name());
    println!("{}", <(i32, (u8, String), Option<u8>)>::name());
    let s = Stack { items: vec!['a', 'b'] };
    println!("{:?}", s.first());
    let t = (1, 2, 3, 4);
    println!("{}", sum_tuple!(t; 0 1 2 3));
}
