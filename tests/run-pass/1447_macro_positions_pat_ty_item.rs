// macros in pattern, type, item, statement, expression positions
macro_rules! pat_range { ($lo:literal, $hi:literal) => { $lo..=$hi }; }
macro_rules! pat_tuple { ($a:pat, $b:pat) => { ($a, $b) }; }
macro_rules! some_of { ($p:pat) => { Some($p) }; }
macro_rules! ty_map { ($k:ty, $v:ty) => { std::collections::BTreeMap<$k, $v> }; }
macro_rules! ty_result { ($t:ty) => { Result<$t, String> }; }
macro_rules! arr_ty { ($t:ty; $n:expr) => { [$t; $n] }; }

macro_rules! impl_trait {
    ($tr:ident for $($t:ty),+ => $body:expr) => {
        $( impl $tr for $t { fn describe(&self) -> String { format!($body, self) } } )+
    };
}

trait Describe { fn describe(&self) -> String; }
impl_trait!(Describe for i32, u8, &str => "<{}>");

macro_rules! stmt_mac {
    ($n:ident += $e:expr) => { $n += $e };
}

struct S;
macro_rules! method_mac { () => { fn m(&self) -> i32 { 77 } }; }
impl S { method_mac!(); }

trait T2 { method_mac!(); }
impl T2 for S { method_mac!(); }

macro_rules! assoc_const { ($n:ident = $v:expr) => { const $n: i32 = $v; }; }
struct C;
impl C { assoc_const!(A = 1); assoc_const!(B = 2); }

fn classify(n: i32) -> &'static str {
    match n {
        pat_range!(0, 9) => "digit",
        pat_range!(10, 99) => "two",
        _ => "big",
    }
}

fn parse(s: &str) -> ty_result!(i32) { s.parse::<i32>().map_err(|e| e.to_string()) }

fn main() {
    println!("{} {} {}", classify(5), classify(50), classify(500));
    match (1, "a") { pat_tuple!(1, "a") => println!("tuple matched"), _ => println!("no") }
    if let some_of!(x @ 1..=3) = Some(2) { println!("some {}", x); }
    let mut m: ty_map!(&str, i32) = Default::default();
    m.insert("k", 1);
    println!("{:?}", m);
    println!("{:?} {:?}", parse("12"), parse("x").is_err());
    let a: arr_ty!(u16; 2 + 1) = [1, 2, 3];
    println!("{:?}", a);
    println!("{} {} {}", 5i32.describe(), 6u8.describe(), "s".describe());
    let mut total = 0;
    stmt_mac!(total += 5);
    stmt_mac!(total += 6);
    println!("{}", total);
    println!("{} {}", S.m(), <S as T2>::m(&S));
    println!("{}", C::A + C::B);
    let v = vec![1, 2, 3];
    let s: i32 = v.iter().map(|x| x * 2).sum::<i32>();
    println!("{}", s);
}

