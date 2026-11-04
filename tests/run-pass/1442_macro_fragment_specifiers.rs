// fragment specifiers: lifetime, vis, literal, meta, path, block, item, stmt, pat_param, ty
use std::fmt::Debug;

macro_rules! with_lt {
    ($lt:lifetime, $name:ident) => {
        struct $name<$lt> { s: &$lt str }
        impl<$lt> $name<$lt> { fn get(&self) -> &$lt str { self.s } }
    };
}
with_lt!('a, Holder);

macro_rules! vis_fn {
    ($v:vis fn $name:ident() -> $r:ty $body:block) => { $v fn $name() -> $r $body };
}
vis_fn!(fn private_one() -> i32 { 1 });
vis_fn!(pub fn public_two() -> i32 { 2 });
vis_fn!(pub(crate) fn crate_three() -> i32 { 3 });

macro_rules! lit_kind {
    ($l:literal) => { format!("{:?}", $l) };
}

macro_rules! attr_on {
    (#[$m:meta] $i:item) => { #[$m] $i };
}
attr_on!(#[derive(Debug, Clone)] struct Pt { x: i32 });
attr_on!(#[allow(dead_code)] fn unused_fn() {});

macro_rules! call_path {
    ($p:path, $($a:expr),*) => { $p($($a),*) };
}

mod inner {
    pub mod deep { pub fn add(a: i32, b: i32) -> i32 { a + b } }
    pub fn neg(a: i32) -> i32 { -a }
}

macro_rules! run_stmts {
    ($($s:stmt;)*) => {{ $($s;)* }};
}

macro_rules! match_pat {
    ($e:expr, $($p:pat_param)|+) => { match $e { $($p)|+ => true, _ => false } };
}

macro_rules! match_pat2021 {
    ($e:expr, $p:pat) => { matches!($e, $p) };
}

macro_rules! typed_vec {
    ($t:ty => $($e:expr),*) => { { let v: Vec<$t> = vec![$($e as $t),*]; v } };
}

macro_rules! fn_type {
    ($t:ty) => { fn(i32) -> $t };
}

macro_rules! show_all {
    ($($e:expr),+) => { $( show(&$e); )+ };
}
fn show<T: Debug>(t: &T) { println!("show: {:?}", t); }

macro_rules! ident_to_string {
    ($i:ident) => { stringify!($i) };
}

macro_rules! block_value {
    ($b:block) => { { let r = $b; r * 2 } };
}

macro_rules! items {
    ($($i:item)*) => { $($i)* };
}
items! {
    const K: u32 = 9;
    static S: &str = "static";
    fn k_plus(x: u32) -> u32 { x + K }
}

fn main() {
    let h = Holder { s: "held" };
    println!("{}", h.get());
    println!("{} {} {}", private_one(), public_two(), crate_three());
    println!("{} {} {} {} {}", lit_kind!(1u8), lit_kind!(-3), lit_kind!("s\n"), lit_kind!('c'), lit_kind!(2.5e1));
    println!("{} {}", lit_kind!(true), lit_kind!(b'x'));
    println!("{:?}", Pt { x: 3 }.clone());
    println!("{} {}", call_path!(inner::deep::add, 2, 3), call_path!(inner::neg, 7));
    run_stmts! { let a = 5; let b = a * 2; println!("stmts {}", a + b); };
    println!("{} {}", match_pat!(3, 1 | 3 | 5), match_pat!(4, 1 | 3 | 5));
    println!("{} {}", match_pat2021!(Some(4), Some(1..=5) | None), match_pat2021!(Some(9), Some(1..=5) | None));
    println!("{:?}", typed_vec!(f64 => 1, 2, 3));
    let f: fn_type!(String) = |x| x.to_string();
    println!("{}", f(12));
    show_all!(1, "two", [3, 4], (5, '6'));
    println!("{}", ident_to_string!(r#match));
    println!("{}", block_value!({ let q = 10; q + 1 }));
    println!("{} {}", k_plus(1), S);
}
