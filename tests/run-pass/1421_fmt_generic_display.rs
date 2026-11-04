use std::collections::BTreeMap;
use std::fmt::{self, Debug, Display, Formatter};
use std::marker::PhantomData;

struct Money<C: Currency> { cents: i64, _c: PhantomData<C> }
trait Currency { const SYM: &'static str; const DECIMALS: usize; }
struct Usd; struct Jpy;
impl Currency for Usd { const SYM: &'static str = "$"; const DECIMALS: usize = 2; }
impl Currency for Jpy { const SYM: &'static str = "¥"; const DECIMALS: usize = 0; }
impl<C: Currency> Money<C> { fn new(c: i64) -> Self { Money { cents: c, _c: PhantomData } } }
impl<C: Currency> Display for Money<C> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = if C::DECIMALS == 0 { format!("{}{}", C::SYM, self.cents) } else {
            let d = 10i64.pow(C::DECIMALS as u32);
            format!("{}{}.{:0w$}", C::SYM, self.cents / d, self.cents % d, w = C::DECIMALS)
        };
        f.pad(&s)
    }
}
impl<C: Currency> Debug for Money<C> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Money").field("cents", &self.cents).field("sym", &C::SYM).finish()
    }
}

struct Raw(i32);
impl Display for Raw { fn fmt(&self, f: &mut Formatter) -> fmt::Result { write!(f, "raw{}", self.0) } }
struct Fwd(f64);
impl Display for Fwd {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if let Some(p) = f.precision() { write!(f, "F<{:.*}>", p, self.0) } else { Display::fmt(&self.0, f) }
    }
}
struct Flags;
impl Display for Flags {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "w={:?} p={:?} alt={} plus={} zero={} fill={:?} align={}", f.width(), f.precision(), f.alternate(), f.sign_plus(), f.sign_aware_zero_pad(), f.fill(),
            match f.align() { Some(fmt::Alignment::Left) => "L", Some(fmt::Alignment::Right) => "R", Some(fmt::Alignment::Center) => "C", None => "-" })
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
enum Shape<T> { Pt, Circle { r: T }, Poly(Vec<(T, T)>) }
#[derive(Debug, Default)]
struct Wrapper<'a, T: Debug> { name: &'a str, items: Vec<T>, map: BTreeMap<u8, Option<T>>, unit: (), tup: (i8, &'a str) }

struct List<T>(Vec<T>);
impl<T: Debug> Debug for List<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result { f.debug_list().entries(self.0.iter()).finish() }
}
struct Set<T>(Vec<T>);
impl<T: Debug> Debug for Set<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result { f.debug_set().entries(&self.0).finish() }
}
struct Pairs<K, V>(Vec<(K, V)>);
impl<K: Debug, V: Debug> Debug for Pairs<K, V> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result { f.debug_map().entries(self.0.iter().map(|(k, v)| (k, v))).finish() }
}
struct Tup;
impl Debug for Tup {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result { f.debug_tuple("Tup").field(&1).field(&"x").finish_non_exhaustive() }
}

fn show<T: Display + ?Sized>(t: &T) -> String { format!("[{:>10}] [{:<10}] [{:^10}] [{:*^9}]", t, t, t, t) }

fn main() {
    let a: Money<Usd> = Money::new(12345);
    let b: Money<Jpy> = Money::new(500);
    println!("{}", show(&a));
    println!("{}", show(&b));
    println!("{:?}", a);
    println!("{:#?}", b);
    println!("{}", show(&Raw(7)));
    println!("[{:8.2}] [{:8}] [{:.0}]", Fwd(3.14159), Fwd(2.5), Fwd(9.99));
    println!("{}", Flags);
    println!("{:+#012.3}", Flags);
    println!("{:_<7}", Flags);
    println!("{:>w$.p$}", Flags, w = 4, p = 1);
    let shapes = vec![Shape::Pt, Shape::Circle { r: 1.5 }, Shape::Poly(vec![(0.0, 1.0), (2.0, 3.0)])];
    println!("{:?}", shapes);
    println!("{:#?}", shapes[2]);
    println!("{} {}", Shape::Pt < Shape::Circle { r: 0.0 }, Shape::Circle { r: 2.0 } > Shape::Circle { r: 1.0 });
    println!("{:?}", Shape::Poly(vec![(1, 2)]).partial_cmp(&Shape::Poly(vec![(1, 3)])));
    let mut m = BTreeMap::new();
    m.insert(1, Some(vec!['a']));
    m.insert(2, None);
    let w = Wrapper { name: "w", items: vec![vec!['z']], map: m, unit: (), tup: (-1, "t") };
    println!("{:?}", w);
    println!("{:#?}", w);
    let d: Wrapper<u8> = Default::default();
    println!("{:?}", d);
    println!("{:?} {:?} {:?} {:?}", List(vec![1, 2]), Set(vec!["a"]), Pairs(vec![(1, "one"), (2, "two")]), Tup);
    println!("{:#?}", Pairs(vec![("k", List(vec![Some(1)]))]));
    println!("{:#?}", Tup);
    println!("[{:>6?}] [{:<8?}] [{:^9?}]", Some(1), "ab", (1, 2));
    println!("[{:5}] [{:<5}] [{:^5}] [{:>5}]", true, 'x', "é", 1.5f32);
    println!("{:#x} {:#o} {:#b} {:08.3e} {:e}", 255, 8, 5u8, 1234.5, 0.00012);
}
