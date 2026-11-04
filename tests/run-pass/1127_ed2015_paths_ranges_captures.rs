// edition:2015
// Edition 2015: `use` paths are relative to the crate root (even inside
// nested modules), `::path` means crate root, `extern crate` of sysroot crates,
// `...` inclusive range patterns, whole-variable closure captures (drop order
// differs from 2021), and the non-format single-argument panic!.
extern crate core;
extern crate std as mystd;

use std::cell::RefCell;
use std::panic;

mod util {
    pub fn double(x: i32) -> i32 { x * 2 }
    pub mod deep {
        // 2015: this path is resolved from the crate root, not relative to `deep`
        use util::double;
        use shapes::Kind;
        pub fn quad(x: i32) -> i32 { double(double(x)) }
        pub fn kind_name(k: Kind) -> &'static str {
            match k { Kind::Small => "small", Kind::Large => "large" }
        }
        pub fn via_root(x: i32) -> i32 { ::util::double(x) + ::shapes::BASE }
    }
}

mod shapes {
    pub const BASE: i32 = 1000;
    #[derive(Clone, Copy, Debug)]
    pub enum Kind { Small, Large }
    pub fn classify(n: u32) -> Kind {
        match n {
            0...9 => Kind::Small,
            _ => Kind::Large,
        }
    }
}

fn grade(score: u8) -> char {
    match score {
        90...100 => 'A',
        80...89 => 'B',
        70...79 => 'C',
        0...69 => 'F',
        _ => '?',
    }
}

thread_local!(static LOG: RefCell<Vec<String>> = RefCell::new(Vec::new()));

struct Loud(&'static str);
impl Drop for Loud {
    fn drop(&mut self) {
        LOG.with(|l| l.borrow_mut().push(format!("drop {}", self.0)));
    }
}

struct Pair { a: Loud, b: Loud }

fn capture_demo() {
    let p = Pair { a: Loud("a"), b: Loud("b") };
    {
        // In 2015/2018 the closure captures all of `p` (by move), so both
        // fields are dropped when the closure is dropped. In 2021 only `p.a`
        // would be captured and `p.b` would be dropped at the end of the fn.
        let c = move || {
            let _x = &p.a;
            LOG.with(|l| l.borrow_mut().push("closure ran".to_string()));
        };
        c();
        LOG.with(|l| l.borrow_mut().push("closure about to drop".to_string()));
    }
    LOG.with(|l| l.borrow_mut().push("end of capture_demo".to_string()));
}

fn main() {
    println!("{} {} {}", util::double(4), util::deep::quad(3), util::deep::via_root(5));
    for n in [3u32, 10].iter() {
        println!("{} -> {}", n, util::deep::kind_name(shapes::classify(*n)));
    }
    let grades: String = [100u8, 95, 85, 79, 12, 101].iter().map(|&s| grade(s)).collect();
    println!("grades: {}", grades);
    println!("core max: {}  mystd min: {}", core::cmp::max(3, 9), mystd::cmp::min(3, 9));

    capture_demo();
    LOG.with(|l| {
        for line in l.borrow().iter() {
            println!("  {}", line);
        }
    });

    panic::set_hook(Box::new(|_| {}));
    let r = panic::catch_unwind(|| {
        panic!("{}");
    });
    let payload = r.unwrap_err();
    let msg = payload.downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| payload.downcast_ref::<String>().cloned());
    println!("panic payload: {:?}", msg);
    let r2 = panic::catch_unwind(|| {
        panic!("value {}", 42);
    });
    let p2 = r2.unwrap_err();
    println!("formatted payload: {:?}", p2.downcast_ref::<String>());
}
