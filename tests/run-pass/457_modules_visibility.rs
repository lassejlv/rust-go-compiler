// Nested modules with pub, pub(crate), pub(super), pub(in path) visibility, private fields with
// constructor functions, re-exports with `pub use` and renaming, glob imports, `self`/`super`/`crate`
// paths, same-named items in different modules, and impl blocks in a different module than the type.
mod geometry {
    pub mod shapes {
        #[derive(Debug)]
        pub struct Circle {
            pub r: f64,
            id: u32,
        }
        impl Circle {
            pub fn new(r: f64) -> Self {
                Circle { r, id: super::next_id() }
            }
            pub fn id(&self) -> u32 {
                self.id
            }
        }

        pub(crate) fn unit() -> Circle {
            Circle::new(1.0)
        }

        pub(super) fn secret_scale() -> f64 {
            2.0
        }

        pub mod detail {
            pub(in crate::geometry) fn precise_pi() -> f64 {
                std::f64::consts::PI
            }
            pub fn area(c: &super::Circle) -> f64 {
                precise_pi() * c.r * c.r
            }
        }
    }

    use std::cell::Cell;
    thread_local!(static COUNTER: Cell<u32> = Cell::new(100));

    fn next_id() -> u32 {
        COUNTER.with(|c| {
            c.set(c.get() + 1);
            c.get()
        })
    }

    pub fn scaled_area(c: &shapes::Circle) -> f64 {
        shapes::detail::area(c) * shapes::secret_scale() / shapes::detail::precise_pi()
    }

    pub use self::shapes::Circle as Round;
    pub use shapes::detail::area;
}

mod util {
    pub fn describe() -> &'static str {
        "util::describe"
    }
    pub mod strings {
        pub fn describe() -> &'static str {
            "util::strings::describe"
        }
        pub fn shout(s: &str) -> String {
            format!("{}!", s.to_uppercase())
        }
        pub fn parent() -> &'static str {
            super::describe()
        }
        pub fn root() -> String {
            crate::top_level()
        }
    }
    pub(crate) mod math {
        pub const SCALE: i32 = 3;
        pub fn triple(x: i32) -> i32 {
            x * SCALE
        }
        pub(crate) struct Acc(pub(crate) i32);
        impl Acc {
            pub fn add(&mut self, v: i32) -> &mut Self {
                self.0 += v;
                self
            }
        }
    }
}

mod ext {
    // extend a type from another module
    impl crate::geometry::shapes::Circle {
        pub fn diameter(&self) -> f64 {
            self.r * 2.0
        }
    }
    pub trait Describe {
        fn describe(&self) -> String;
    }
    impl Describe for crate::geometry::Round {
        fn describe(&self) -> String {
            format!("circle#{} r={}", self.id(), self.r)
        }
    }
    impl Describe for i32 {
        fn describe(&self) -> String {
            format!("int {}", self)
        }
    }
}

fn top_level() -> String {
    "crate::top_level".to_string()
}

use ext::Describe;
use geometry::shapes::{self, detail::area as circle_area};
use util::math::*;
use util::strings::{self as s, shout};

fn main() {
    let c = shapes::Circle::new(2.0);
    let u = shapes::unit();
    let r = geometry::Round::new(0.5);
    println!("{:?} {:?} {:?}", c, u.id(), r.id());
    println!("{:.4} {:.4} {:.4}", circle_area(&c), geometry::area(&u), geometry::scaled_area(&r));
    println!("{} {}", c.diameter(), r.describe());
    println!("{}", 7.describe());
    println!("{} | {} | {} | {}", util::describe(), s::describe(), s::parent(), s::root());
    println!("{}", shout("quiet"));
    println!("{} {}", triple(SCALE), util::math::triple(-2));
    let mut acc = Acc(0);
    acc.add(5).add(10).add(-3);
    println!("acc {}", acc.0);
    let v: Vec<geometry::Round> = (1..=3).map(|i| geometry::Round::new(i as f64)).collect();
    let ids: Vec<u32> = v.iter().map(|c| c.id()).collect();
    println!("{:?}", ids);
    let total: f64 = v.iter().map(geometry::scaled_area).sum();
    println!("{}", total);
}
