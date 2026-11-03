// nested modules: pub(crate), pub(super), pub(in path), re-exports, glob imports, use-renames, impls across modules
mod geometry {
    pub mod shapes {
        #[derive(Debug, Clone)]
        pub struct Rect {
            pub w: f64,
            pub h: f64,
            pub(crate) id: u32,
            pub(super) tag: &'static str,
            secret: u8,
        }
        impl Rect {
            pub fn new(w: f64, h: f64) -> Self {
                Rect { w, h, id: next_id(), tag: "rect", secret: 7 }
            }
            pub fn secret_doubled(&self) -> u8 {
                self.secret * 2
            }
        }
        fn next_id() -> u32 {
            use std::sync::atomic::{AtomicU32, Ordering};
            static N: AtomicU32 = AtomicU32::new(100);
            N.fetch_add(1, Ordering::Relaxed)
        }
        pub(in crate::geometry) fn internal_scale(r: &mut Rect, k: f64) {
            r.w *= k;
            r.h *= k;
        }
        pub mod deep {
            pub fn whoami() -> String {
                format!("{}::deep", super::super::NAME)
            }
            pub(crate) fn parent_tag(r: &super::Rect) -> &'static str {
                r.tag
            }
        }
    }

    pub const NAME: &str = "geometry";

    pub fn tag_of(r: &shapes::Rect) -> &'static str {
        r.tag
    }

    pub fn scaled(mut r: shapes::Rect, k: f64) -> shapes::Rect {
        shapes::internal_scale(&mut r, k);
        r
    }

    pub use self::shapes::Rect as Rectangle;
    pub use shapes::deep::whoami;

    pub trait Area {
        fn area(&self) -> f64;
        fn describe(&self) -> String {
            format!("area={:.1}", self.area())
        }
    }
}

mod impls {
    use crate::geometry::{Area, Rectangle};
    impl Area for Rectangle {
        fn area(&self) -> f64 {
            self.w * self.h
        }
    }
    impl Rectangle {
        pub fn perimeter(&self) -> f64 {
            2.0 * (self.w + self.h)
        }
    }
    pub(crate) mod helpers {
        pub fn total<T: super::Area>(items: &[T]) -> f64 {
            items.iter().map(|i| i.area()).sum()
        }
    }
}

mod prelude {
    pub use crate::geometry::Area as _;
    pub use crate::geometry::{scaled, Rectangle};
    pub use crate::impls::helpers::*;
}

mod counter {
    pub struct Counter {
        count: u32,
    }
    impl Counter {
        pub const fn new() -> Self {
            Counter { count: 0 }
        }
        pub fn inc(&mut self) -> u32 {
            self.count += 1;
            self.count
        }
    }
    impl Default for Counter {
        fn default() -> Self {
            Self::new()
        }
    }
    pub mod nested {
        pub fn peek(c: &super::Counter) -> u32 {
            c.count
        }
    }
}

mod shadow {
    pub fn f() -> &'static str {
        "shadow::f"
    }
    pub mod inner {
        pub fn f() -> &'static str {
            "inner::f"
        }
        pub fn call_both() -> String {
            format!("{} {} {}", f(), self::f(), super::f())
        }
    }
}

use prelude::*;
use shadow::inner::{self as si, call_both};

fn f() -> &'static str {
    "crate::f"
}

fn main() {
    let r = Rectangle::new(2.0, 3.0);
    let r2 = geometry::shapes::Rect::new(1.0, 1.0);
    println!("{} {} {}", r.area(), r.describe(), r.perimeter());
    println!("ids {} {}", r.id, r2.id);
    println!("{}", geometry::tag_of(&r));
    println!("{}", geometry::shapes::deep::parent_tag(&r2));
    println!("{}", r.secret_doubled());
    let big = scaled(r.clone(), 10.0);
    println!("{} {}", big.w, big.area());
    println!("{}", total(&[r, r2, big]));
    println!("{}", geometry::whoami());
    let mut c = counter::Counter::default();
    c.inc();
    c.inc();
    println!("{} {}", c.inc(), counter::nested::peek(&c));
    println!("{} | {} | {} | {}", f(), si::f(), call_both(), crate::shadow::f());
    {
        use geometry::NAME as N;
        fn f() -> &'static str {
            "block::f"
        }
        println!("{} {}", N, f());
    }
    static COUNTER: std::sync::Mutex<counter::Counter> = std::sync::Mutex::new(counter::Counter::new());
    for _ in 0..3 {
        COUNTER.lock().unwrap().inc();
    }
    println!("{}", counter::nested::peek(&COUNTER.lock().unwrap()));
}
