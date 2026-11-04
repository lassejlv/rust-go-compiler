// cfg / cfg_attr interplay with macros and derives
#[cfg_attr(all(), derive(Debug, Clone))]
struct A { x: i32 }

#[cfg_attr(any(), derive(Debug))]
#[derive(Clone)]
struct B;

#[derive(Debug)]
struct C {
    a: i32,
    #[cfg(any())]
    b: i32,
    #[cfg(all())]
    c: i32,
}

#[derive(Debug, PartialEq)]
enum D {
    X,
    #[cfg(any())]
    Y,
    Z(#[cfg(all())] u8, #[cfg(any())] u16),
}

macro_rules! cfg_items {
    ($(#[$m:meta] $i:item)*) => { $( #[$m] $i )* };
}

cfg_items! {
    #[cfg(target_pointer_width = "64")]
    fn ptr_width() -> u32 { 64 }
    #[cfg(not(target_pointer_width = "64"))]
    fn ptr_width() -> u32 { 32 }
    #[cfg(unix)]
    fn family() -> &'static str { "unix" }
    #[cfg(not(unix))]
    fn family() -> &'static str { "other" }
}

macro_rules! when {
    (debug => $a:expr, release => $b:expr) => {{
        #[cfg(debug_assertions)] { $a }
        #[cfg(not(debug_assertions))] { $b }
    }};
}

fn main() {
    println!("{:?}", A { x: 1 }.clone());
    let _b = B.clone();
    println!("{:?}", C { a: 1, c: 3 });
    println!("{:?} {:?}", D::X, D::Z(4));
    println!("{} {}", ptr_width(), family());
    let mode = when!(debug => "debug", release => "release");
    println!("{}", mode.len() > 0);
    let v = [1, 2, 3];
    let total: i32 = v.iter().map(|x| {
        #[cfg(any())] { return 0; }
        x * 2
    }).sum();
    println!("{}", total);
    #[cfg(any())]
    println!("hidden");
    #[cfg(all())]
    println!("shown");
    let t = (#[cfg(all())] 1, #[cfg(any())] 2, 3);
    println!("{:?}", t);
    let arr = [#[cfg(any())] 0, 1, 2];
    println!("{:?}", arr);
}
