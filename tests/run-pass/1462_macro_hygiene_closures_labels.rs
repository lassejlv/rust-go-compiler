// more hygiene: nested macros, closures, labels, identifiers passed in vs created
macro_rules! with_counter {
    ($body:expr) => {{
        let mut counter = 0;
        for _ in 0..3 { counter += 1; }
        let r = $body;
        (counter, r)
    }};
}

macro_rules! using_ident {
    ($i:ident, $body:expr) => {{ let $i = 10; $body }};
}

macro_rules! twice_label {
    ($b:block) => { 'l: for _ in 0..2 { $b; continue 'l; } };
}

macro_rules! make_closure {
    ($x:ident) => { move |y: i32| $x + y };
}

macro_rules! outer {
    ($e:expr) => {{ let v = 1; inner!(v, $e) }};
}
macro_rules! inner {
    ($a:ident, $e:expr) => {{ let v = 100; $a + $e + v }};
}

macro_rules! gen_static {
    ($name:ident) => {
        static $name: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    };
}
gen_static!(HITS);

macro_rules! bump { () => { HITS.fetch_add(1, std::sync::atomic::Ordering::SeqCst) }; }

macro_rules! struct_lit {
    ($t:ident { $($f:ident),* }) => { $t { $($f),* } };
}

#[derive(Debug)]
struct P { a: i32, b: i32 }

macro_rules! let_tuple {
    ($($n:ident),* = $e:expr) => { let ($($n),*) = $e; };
}

fn main() {
    let counter = 50;
    println!("{:?}", with_counter!(counter * 2));
    let x = 5;
    println!("{}", using_ident!(x, x + 1));
    println!("{}", x);
    let mut n = 0;
    twice_label!({ n += 1 });
    println!("{}", n);
    let base = 7;
    let f = make_closure!(base);
    println!("{}", f(3));
    let v = 1000;
    println!("{}", outer!(v));
    bump!(); bump!();
    println!("{}", HITS.load(std::sync::atomic::Ordering::SeqCst));
    let a = 1; let b = 2;
    println!("{:?}", struct_lit!(P { a, b }));
    let_tuple!(q, r, s = (1, "two", 3.0));
    println!("{} {} {}", q, r, s);
}
