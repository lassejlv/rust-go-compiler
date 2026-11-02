// Macros using $crate paths, macros defined inside modules with #[macro_export]-like usage via
// `pub(crate) use`, macros calling helper functions in other modules, and hygiene.
mod util {
    pub fn join_parts(parts: &[String]) -> String {
        parts.join("::")
    }

    pub mod log {
        use std::cell::RefCell;
        thread_local! {
            pub static LINES: RefCell<Vec<String>> = RefCell::new(Vec::new());
        }
        pub fn push(level: Level, msg: String) {
            LINES.with(|l| l.borrow_mut().push(format!("[{:?}] {}", level, msg)));
        }
        pub fn drain() -> Vec<String> {
            LINES.with(|l| std::mem::take(&mut *l.borrow_mut()))
        }
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
        pub enum Level {
            Debug,
            Info,
            Warn,
        }
        pub static mut MIN: Level = Level::Debug;
        pub fn enabled(l: Level) -> bool {
            unsafe { l >= MIN }
        }
    }

    #[macro_export]
    macro_rules! log_at {
        ($lvl:ident, $($arg:tt)+) => {
            if $crate::util::log::enabled($crate::util::log::Level::$lvl) {
                $crate::util::log::push($crate::util::log::Level::$lvl, format!($($arg)+));
            }
        };
    }

    macro_rules! mkpath {
        ($($seg:ident)::+) => {
            $crate::util::join_parts(&[$(stringify!($seg).to_string()),+])
        };
    }
    pub(crate) use mkpath;
}

mod shapes {
    pub trait Area {
        fn area(&self) -> f64;
    }

    macro_rules! shape {
        ($name:ident { $($f:ident),* } => |$s:ident| $body:expr) => {
            #[derive(Debug)]
            pub struct $name { $(pub $f: f64),* }
            impl $crate::shapes::Area for $name {
                fn area(&self) -> f64 { let $s = self; $body }
            }
        };
    }

    shape!(Rect { w, h } => |s| s.w * s.h);
    shape!(Circle { r } => |s| 3.0 * s.r * s.r);
    shape!(Tri { a, b, c } => |s| {
        let p = (s.a + s.b + s.c) / 2.0;
        (p * (p - s.a) * (p - s.b) * (p - s.c)).sqrt()
    });
}

// Hygiene: the macro's `tmp` does not clash with the caller's `tmp`
macro_rules! swap_add {
    ($a:expr, $b:expr) => {{
        let tmp = $a;
        $a = $b;
        $b = tmp + $b;
    }};
}

// A macro that defines a function and a constant visible to the caller
macro_rules! define_scale {
    ($fname:ident, $cname:ident, $factor:expr) => {
        const $cname: i32 = $factor;
        fn $fname(x: i32) -> i32 {
            x * $cname
        }
    };
}
define_scale!(triple, TRIPLE, 3);
define_scale!(tenfold, TENFOLD, 10);

macro_rules! maybe {
    ($e:expr) => {
        match $e {
            Some(v) => v,
            None => return None,
        }
    };
}

fn first_even_doubled(v: &[i32]) -> Option<i32> {
    let x = maybe!(v.iter().find(|x| *x % 2 == 0));
    Some(x * 2)
}

use shapes::Area;

fn main() {
    log_at!(Info, "starting {} v{}", "app", 1);
    log_at!(Debug, "debug detail {:?}", (1, 2));
    unsafe {
        util::log::MIN = util::log::Level::Info;
    }
    log_at!(Debug, "suppressed");
    log_at!(Warn, "careful: {}", 42);
    for l in util::log::drain() {
        println!("{}", l);
    }
    println!("{}", util::mkpath!(std::collections::HashMap));
    println!("{}", util::mkpath!(single));

    let shapes: Vec<Box<dyn Area>> = vec![
        Box::new(shapes::Rect { w: 2.0, h: 3.5 }),
        Box::new(shapes::Circle { r: 2.0 }),
        Box::new(shapes::Tri { a: 3.0, b: 4.0, c: 5.0 }),
    ];
    for s in &shapes {
        println!("area {:.2}", s.area());
    }
    println!("{:?}", shapes::Rect { w: 1.0, h: 1.0 });

    let tmp = 100;
    let mut a = 1;
    let mut b = 2;
    swap_add!(a, b);
    println!("a={} b={} tmp={}", a, b, tmp);

    println!("{} {} {} {}", triple(7), tenfold(7), TRIPLE, TENFOLD);
    println!("{:?} {:?}", first_even_doubled(&[1, 3, 4, 6]), first_even_doubled(&[1, 3]));
}
