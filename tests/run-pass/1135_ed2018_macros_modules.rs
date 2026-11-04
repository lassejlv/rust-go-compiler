// edition:2018
// Edition 2018 module/macro system: #[macro_export] macros imported with
// `use crate::name`, `$crate` paths inside macros, macros defined in modules
// and re-exported, glob imports with shadowing, enum variant imports,
// `use` of associated items through `Self`, and nested `use` trees.
pub mod registry {
    use std::collections::BTreeMap;

    #[derive(Debug, Default)]
    pub struct Registry { pub(crate) items: BTreeMap<String, Entry> }

    #[derive(Debug, Clone, PartialEq)]
    pub enum Entry { Num(i64), Text(String), List(Vec<Entry>) }

    impl Registry {
        pub fn put(&mut self, k: &str, e: Entry) -> &mut Self {
            self.items.insert(k.to_string(), e);
            self
        }
        pub fn total(&self) -> i64 {
            fn sum(e: &Entry) -> i64 {
                match e {
                    Entry::Num(n) => *n,
                    Entry::Text(t) => t.len() as i64,
                    Entry::List(v) => v.iter().map(sum).sum(),
                }
            }
            self.items.values().map(sum).sum()
        }
    }

    #[macro_export]
    macro_rules! entry {
        ([$($e:tt),*]) => { $crate::registry::Entry::List(vec![$($crate::entry!($e)),*]) };
        ($n:literal) => { $crate::registry::Entry::from($n) };
    }

    impl From<i64> for Entry { fn from(n: i64) -> Self { Entry::Num(n) } }
    impl From<&str> for Entry { fn from(s: &str) -> Self { Entry::Text(s.to_string()) } }

    macro_rules! local_helper {
        ($x:expr) => { $x * 100 };
    }
    pub(crate) use local_helper;
}

mod shapes {
    pub mod round {
        #[derive(Debug)]
        pub struct Circle(pub f64);
        pub fn area(c: &Circle) -> f64 { 3.0 * c.0 * c.0 }
    }
    pub mod angular {
        #[derive(Debug)]
        pub struct Square(pub f64);
        pub fn area(s: &Square) -> f64 { s.0 * s.0 }
    }
    pub use self::angular::Square;
    pub use self::round::{area as circle_area, Circle};
}

mod prelude_like {
    pub fn describe() -> &'static str { "glob" }
    pub fn shadowed() -> &'static str { "from glob" }
}

mod consumer {
    use crate::entry;
    use crate::registry::Entry;
    pub fn make() -> Entry { entry!([10, [20, "abc"]]) }
}
use crate::registry::{local_helper, Entry::{self, *}, Registry};
use prelude_like::*;
use shapes::{angular::area as square_area, circle_area, Circle, Square};

fn shadowed() -> &'static str { "local wins" }

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Level { Low = 1, Mid = 5, High = 10 }

impl Level {
    const ALL: [Self; 3] = [Self::Low, Self::Mid, Self::High];
    fn next(self) -> Option<Self> {
        use Level::*;
        match self { Low => Some(Mid), Mid => Some(High), High => None }
    }
}

fn main() {
    let mut r = Registry::default();
    r.put("a", entry!(5)).put("b", entry!("hello")).put("c", entry!([1, "xy", [2, 3]]));
    println!("{:?}", r.items.get("c"));
    println!("total {}", r.total());
    r.put("d", consumer::make());
    println!("total {} {:?}", r.total(), r.items.keys().collect::<Vec<_>>());
    let e: Entry = Num(3);
    let t = Text("x".into());
    println!("{:?} {:?} {}", e, t, matches!(Entry::List(vec![]), List(v) if v.is_empty()));
    println!("helper {}", local_helper!(7));
    println!("{} / {}", describe(), shadowed());
    println!("{} {}", circle_area(&Circle(1.0)), square_area(&Square(3.0)));
    println!("{:?}", Level::ALL.iter().map(|l| (*l as i32, l.next())).collect::<Vec<_>>());
    let mut sorted = vec![Level::High, Level::Low, Level::Mid];
    sorted.sort();
    println!("{:?} max={:?}", sorted, sorted.iter().max());
}
