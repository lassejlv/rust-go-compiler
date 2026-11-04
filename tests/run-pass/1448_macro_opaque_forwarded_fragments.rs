// forwarded fragments are opaque: precedence, re-matching, stringify
macro_rules! times3 { ($e:expr) => { $e * 3 }; }
macro_rules! neg { ($e:expr) => { -$e }; }
macro_rules! fwd { ($e:expr) => { times3!($e) }; }
macro_rules! is_lit_tt { ($t:tt) => { "tt" }; }
macro_rules! show { ($e:expr) => { stringify!($e) }; }
macro_rules! show2 { ($e:expr) => { show!($e) }; }
macro_rules! is_one {
    (1) => { "literal one" };
    ($e:expr) => { "some expr" };
}
macro_rules! fwd_is_one { ($e:expr) => { is_one!($e) }; }
macro_rules! fwd_tt_is_one { ($t:tt) => { is_one!($t) }; }
macro_rules! method_on { ($e:expr) => { $e.len() }; }
macro_rules! call_mac { ($m:ident, $($a:tt)*) => { $m!($($a)*) }; }
macro_rules! as_cast { ($e:expr, $t:ty) => { $e as $t }; }
macro_rules! idx { ($a:expr, $i:expr) => { $a[$i] }; }
macro_rules! ty_fwd { ($t:ty) => { ty_inner!($t) }; }
macro_rules! ty_inner { ($t:ty) => { std::mem::size_of::<$t>() }; }
macro_rules! ident_fwd { ($i:ident) => { ident_match!($i) }; }
macro_rules! ident_match { (foo) => { "is foo" }; ($i:ident) => { "other ident" }; }
macro_rules! lit_fwd { ($l:literal) => { lit_match!($l) }; }
macro_rules! lit_match { ("x") => { "x literal" }; ($l:literal) => { "other literal" }; }
macro_rules! range_mac { ($a:expr, $b:expr) => { ($a..$b).sum::<i32>() }; }
macro_rules! unary_chain { ($e:expr) => { !$e }; }

fn main() {
    println!("{}", times3!(1 + 2));
    println!("{}", fwd!(1 + 2));
    println!("{}", neg!(2 - 5));
    println!("{}", is_lit_tt!(1));
    println!("{}", show!(1 + 2 * 3));
    println!("{}", show2!(a + b));
    println!("{}", is_one!(1));
    println!("{}", fwd_is_one!(1));
    println!("{}", fwd_tt_is_one!(1));
    println!("{}", method_on!("abc".to_string() + "de"));
    println!("{}", call_mac!(times3, 4 + 1));
    println!("{}", as_cast!(300 + 1, u8));
    println!("{}", as_cast!(-1i32, u32));
    let a = [10, 20, 30];
    println!("{}", idx!(a, 1 + 1));
    println!("{}", ty_fwd!((u8, u32)));
    println!("{}", ident_fwd!(foo));
    println!("{}", ident_fwd!(bar));
    println!("{}", lit_fwd!("x"));
    println!("{}", lit_fwd!("y"));
    println!("{}", range_mac!(1, 2 + 3));
    println!("{}", unary_chain!(1 == 2));
    println!("{}", times3!(if true { 2 } else { 3 }));
    println!("{}", times3!({ 5 }));
}
