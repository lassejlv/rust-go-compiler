// derives must not be confused by user items shadowing prelude/std names
#![allow(dead_code)]
type Result<T> = std::result::Result<T, String>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
enum Ordering { #[default] Less, Equal, Greater }

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
struct Formatter { f: u8, state: u8, other: u8, __self_0: u8 }

#[derive(Debug, Clone, PartialEq, Default)]
struct Option { some: bool }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Thing { Some(u8), None, Ok { v: i8 }, Err }

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Arr<T, const N: usize> { items: [T; N] }

trait Clone2 {}
#[derive(Debug, Clone, Default)]
struct Self_ { cmp: i32, partial_cmp: i32, eq: i32, fmt: i32, hash: i32, clone: i32 }

#[derive(Debug, PartialEq, Clone)]
enum Generic<T, E> where E: std::fmt::Debug { A(T), B(E), C }

mod inner {
    #[derive(Debug, Clone, PartialEq, Default)]
    pub struct String { pub s: std::string::String }
    pub fn mk() -> String { String { s: "inner".into() } }
}

fn parse(s: &str) -> Result<i32> { s.parse().map_err(|_| format!("bad {}", s)) }

fn main() {
    println!("{:?} {:?}", parse("3"), parse("z"));
    println!("{:?} {:?} {:?}", Ordering::default(), Ordering::Less.cmp(&Ordering::Greater), Ordering::Equal.max(Ordering::Less));
    let f = Formatter { f: 1, state: 2, other: 3, __self_0: 4 };
    println!("{:?} {}", f, f == f.clone());
    println!("{:?}", f.cmp(&Formatter::default()));
    println!("{:?}", Option::default());
    let ts = [Thing::Some(1), Thing::None, Thing::Ok { v: -1 }, Thing::Err];
    println!("{:?} {}", ts, ts[0] == Thing::Some(1));
    let a: Arr<u8, 3> = Arr { items: [1, 2, 3] };
    println!("{:?} {:?}", a, a.clone() < Arr { items: [1, 2, 4] });
    println!("{:?}", Self_::default().clone());
    let g: Generic<i32, &str> = Generic::B("e");
    println!("{:?} {}", g, g == Generic::C);
    println!("{:?}", inner::mk());
}
