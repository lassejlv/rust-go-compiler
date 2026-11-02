// Macros generating trait impls for tuples of arity 1..8 (like serde, bevy, gpui's ParentElement),
// using a recursive "peel" macro so every prefix arity gets an impl.
use std::fmt::Debug;

trait Describe {
    fn describe(&self) -> Vec<String>;
    const ARITY: usize;
}

trait Encode {
    fn encode(&self, out: &mut Vec<u8>);
}

impl Encode for u8 {
    fn encode(&self, out: &mut Vec<u8>) {
        out.push(*self);
    }
}
impl Encode for u16 {
    fn encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.to_le_bytes());
    }
}
impl Encode for i32 {
    fn encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.to_le_bytes());
    }
}
impl Encode for bool {
    fn encode(&self, out: &mut Vec<u8>) {
        out.push(*self as u8);
    }
}
impl Encode for &str {
    fn encode(&self, out: &mut Vec<u8>) {
        out.push(self.len() as u8);
        out.extend_from_slice(self.as_bytes());
    }
}
impl Encode for char {
    fn encode(&self, out: &mut Vec<u8>) {
        let mut b = [0; 4];
        out.extend_from_slice(self.encode_utf8(&mut b).as_bytes());
    }
}

macro_rules! count {
    () => { 0 };
    ($x:ident $($xs:ident)*) => { 1 + count!($($xs)*) };
}

macro_rules! impl_tuples {
    // peel off the last ident each step so we impl all arities
    ($($name:ident)+) => {
        impl_tuples!(@impl $($name)+);
        impl_tuples!(@peel [] $($name)+);
    };
    (@peel [$($done:ident)*] $last:ident) => {};
    (@peel [$($done:ident)*] $head:ident $($rest:ident)+) => {
        impl_tuples!(@peel_inner [$($done)* $head] $($rest)+);
    };
    (@peel_inner [$($keep:ident)+] $last:ident) => {
        impl_tuples!($($keep)+);
    };
    (@peel_inner [$($keep:ident)+] $next:ident $($rest:ident)+) => {
        impl_tuples!(@peel_inner [$($keep)+ $next] $($rest)+);
    };
    (@impl $($name:ident)+) => {
        #[allow(non_snake_case)]
        impl<$($name: Debug),+> Describe for ($($name,)+) {
            fn describe(&self) -> Vec<String> {
                let ($($name,)+) = self;
                vec![$(format!("{:?}", $name)),+]
            }
            const ARITY: usize = count!($($name)+);
        }
        #[allow(non_snake_case)]
        impl<$($name: Encode),+> Encode for ($($name,)+) {
            fn encode(&self, out: &mut Vec<u8>) {
                let ($($name,)+) = self;
                $( $name.encode(out); )+
            }
        }
    };
}

impl_tuples!(A B C D E F G H);

// A second style: explicit index lists, like std's tuple impls
trait Sum {
    fn sum_all(&self) -> i64;
}
macro_rules! tuple_sum {
    ($( ( $($idx:tt : $T:ident),+ ) )+) => {
        $(
            impl<$($T: Copy + Into<i64>),+> Sum for ($($T,)+) {
                fn sum_all(&self) -> i64 { 0 $( + self.$idx.into() )+ }
            }
        )+
    };
}
tuple_sum! {
    (0: A)
    (0: A, 1: B)
    (0: A, 1: B, 2: C)
    (0: A, 1: B, 2: C, 3: D)
}

fn show<T: Describe>(t: T) {
    println!("arity {} -> {:?}", T::ARITY, t.describe());
}

fn enc<T: Encode>(t: T) -> Vec<u8> {
    let mut v = vec![];
    t.encode(&mut v);
    v
}

fn main() {
    show((1,));
    show((1, "two"));
    show((1, 'c', 3.5));
    show((1u8, 2u16, 3u32, 4u64));
    show(((), [1, 2], Some(3), "x", 5));
    show((1, 2, 3, 4, 5, 6));
    show((1, 2, 3, 4, 5, 6, 7));
    show(('a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'));
    println!("{:?}", enc((7u8,)));
    println!("{:?}", enc((1u8, 258u16, true)));
    println!("{:?}", enc(("hi", -1i32, 'é')));
    println!("{:?}", enc((1u8, 2u8, 3u8, 4u8, 5u8, 6u8, 7u8, (8u8, 9u16))));
    println!("{} {} {}", (5i32,).sum_all(), (1u8, 2i16).sum_all(), (1u32, -2i32, 3u8, 4i64).sum_all());
    println!("{}", <(u8, u8, u8) as Describe>::ARITY);
}
