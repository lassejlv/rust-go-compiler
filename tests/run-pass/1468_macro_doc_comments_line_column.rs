// concat!/stringify!/line!/column! corner cases, macros in array lengths and const generics
macro_rules! loc { () => { (line!(), column!()) }; }
macro_rules! loc2 { () => { loc!() }; }
macro_rules! here_str { () => { concat!(line!(), ":", column!()) }; }
macro_rules! count { () => { 0 }; ($x:tt $($r:tt)*) => { 1 + count!($($r)*) }; }
macro_rules! in_mod { () => { module_path!() }; }

mod a {
    pub mod b {
        pub fn where_am_i() -> &'static str { in_mod!() }
    }
}

struct Buf<const N: usize> { data: [u8; N] }

macro_rules! doc_fn {
    ($(#[$m:meta])* fn $n:ident) => { stringify!($(#[$m])* fn $n() {}) };
}

fn main() {
    println!("{}", concat!(1.50, "|", 0x10, "|", 1_000, "|", 'a', "|", 1u8, "|", -7, "|", 2.5e3, "|", false));
    println!("{}", concat!("a", concat!("b", "c"), stringify!(d e)));
    println!("{:?}", loc!());
    println!("{:?}", loc2!());
    println!("{}", here_str!());
    let (l, c) = loc!(
    );
    println!("{} {}", l, c);
    let arr = [0u8; count!(a b c d)];
    println!("{}", arr.len());
    let b = Buf::<{ count!(x y) }> { data: [1, 2] };
    println!("{:?}", b.data);
    println!("{}", a::b::where_am_i());
    println!("{}", stringify!(
        /// doc line
        fn documented() {}
    ));
    println!("{}", doc_fn!(
        /// outer doc
        #[inline]
        fn f
    ));
    println!("{}", stringify!(//! inner doc
    ));
    println!("{}", stringify!(/** block */ x));
    println!("{}", concat!(line!(), column!()));
    println!("{}", stringify!(a /* comment */ b // line comment
        c));
}
