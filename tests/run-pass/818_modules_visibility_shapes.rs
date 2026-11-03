mod geometry {
    pub mod units {
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
        pub struct Meters(pub f64);
        impl Meters { pub(crate) fn raw(self) -> f64 { self.0 } }
        pub(in crate::geometry) fn scale(m: Meters, k: f64) -> Meters { Meters(m.0 * k) }
        pub const ZERO: Meters = Meters(0.0);
    }
    pub mod shapes {
        use super::units::{self, Meters};
        #[derive(Debug)]
        pub struct Square { side: Meters, pub label: String }
        impl Square {
            pub fn new(side: f64, label: &str) -> Self { Square { side: Meters(side), label: label.into() } }
            pub fn area(&self) -> f64 { self.side.raw() * self.side.raw() }
            pub fn grow(&mut self, k: f64) { self.side = units::scale(self.side, k); }
            pub fn side(&self) -> Meters { self.side }
        }
        pub(super) fn secret_count() -> usize { helpers::COUNT }
        mod helpers { pub(in crate::geometry::shapes) const COUNT: usize = 3; }
    }
    pub use self::shapes::Square as Sq;
    pub fn describe() -> String { format!("geometry with {} helpers, zero={:?}", shapes::secret_count(), units::ZERO) }
}

mod bank {
    use std::collections::BTreeMap;
    #[derive(Debug)]
    pub struct Bank { accounts: BTreeMap<u32, i64>, next: u32 }
    #[derive(Debug, PartialEq)]
    pub enum Error { NoAccount(u32), Insufficient { need: i64, have: i64 } }
    impl Bank {
        pub fn new() -> Self { Bank { accounts: BTreeMap::new(), next: 100 } }
        pub fn open(&mut self, initial: i64) -> u32 { let id = self.next; self.next += 1; self.accounts.insert(id, initial); id }
        fn get_mut(&mut self, id: u32) -> Result<&mut i64, Error> { self.accounts.get_mut(&id).ok_or(Error::NoAccount(id)) }
        pub fn withdraw(&mut self, id: u32, amt: i64) -> Result<i64, Error> {
            let bal = self.get_mut(id)?;
            if *bal < amt { return Err(Error::Insufficient { need: amt, have: *bal }); }
            *bal -= amt;
            Ok(*bal)
        }
        pub fn transfer(&mut self, from: u32, to: u32, amt: i64) -> Result<(), Error> {
            self.get_mut(to)?;
            self.withdraw(from, amt)?;
            *self.get_mut(to)? += amt;
            Ok(())
        }
        pub fn total(&self) -> i64 { self.accounts.values().sum() }
        pub fn report(&self) -> Vec<String> { self.accounts.iter().map(|(k, v)| format!("{}:{}", k, v)).collect() }
    }
}

mod util {
    pub fn fmt_list<T: std::fmt::Display>(xs: &[T]) -> String { xs.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", ") }
    pub mod math {
        pub fn gcd(a: u64, b: u64) -> u64 { if b == 0 { a } else { gcd(b, a % b) } }
        pub fn lcm(a: u64, b: u64) -> u64 { a / gcd(a, b) * b }
        pub(crate) fn lcm_all(xs: &[u64]) -> u64 { xs.iter().copied().fold(1, lcm) }
        pub fn call_parent() -> String { super::fmt_list(&[gcd(12, 18), lcm(4, 6)]) }
    }
}

use geometry::shapes::Square;
use geometry::Sq as AliasSquare;
use util::math::{self as m, gcd};

fn main() {
    let mut s = Square::new(2.0, "first");
    println!("{} area {}", s.label, s.area());
    s.grow(1.5);
    println!("grown side {:?} area {}", s.side(), s.area());
    let t: AliasSquare = AliasSquare::new(1.0, "alias");
    println!("{:?}", t);
    println!("{}", geometry::describe());
    println!("{}", geometry::units::Meters(3.0) > geometry::units::ZERO);
    let mut b = bank::Bank::new();
    let a1 = b.open(500);
    let a2 = b.open(20);
    println!("{:?}", b.withdraw(a1, 100));
    println!("{:?}", b.withdraw(a2, 100));
    println!("{:?}", b.transfer(a1, a2, 250));
    println!("{:?}", b.transfer(a1, 999, 1));
    println!("{:?}", b.transfer(a2, a1, 1000));
    println!("{:?} total={}", b.report(), b.total());
    println!("{} {} {} {}", gcd(84, 36), m::lcm(21, 6), m::lcm_all(&[2, 3, 4, 5, 6, 7]), m::call_parent());
    println!("{}", util::fmt_list(&["x", "y"]));
    println!("{}", crate::util::math::gcd(1071, 462));
}
