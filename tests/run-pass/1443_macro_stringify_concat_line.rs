// stringify!/concat!/line!/column!/file!/module_path! outputs
macro_rules! here {
    () => { (line!(), column!()) };
}

macro_rules! strfy {
    ($($t:tt)*) => { stringify!($($t)*) };
}

mod outer {
    pub mod inner {
        pub fn path() -> &'static str { module_path!() }
        macro_rules! mp { () => { module_path!() } }
        pub fn path2() -> &'static str { mp!() }
    }
}

fn main() {
    println!("{}", stringify!(a + b * c));
    println!("{}", stringify!(Vec<Option<i32>>));
    println!("{}", stringify!(fn foo(x: &mut i32) -> bool { x > 0 }));
    println!("{}", stringify!(a::b::<c>::d));
    println!("{}", stringify!(#[attr] struct S;));
    println!("{}", stringify!(x => y, 'a: loop {}));
    println!("{}", stringify!(-1 .. =2));
    println!("{}", strfy!(1+1   ==   2 && !false));
    println!("{}", strfy!(&&x || &mut y));
    println!("{}", strfy!(a.b().c[0]?));
    println!("{}", strfy!("str" 'c' b"bytes" r"raw" 1_000u32 0x1F));
    println!("{}", strfy!(|x| x + 1));
    println!("{}", strfy!($ crate));
    println!("{}", concat!("a", 1, 2.5, true, 'c', -3));
    println!("{}", concat!());
    println!("{}", concat!("line ", line!()));
    let (l, c) = here!();
    println!("{} {}", l, c);
    println!("{}", column!());
        println!("{}", column!());
    println!("{}", line!());
    println!("{}", file!().ends_with(".rs"));
    println!("{}", module_path!());
    println!("{}", outer::inner::path());
    println!("{}", outer::inner::path2());
    let s: &'static str = concat!(stringify!(x), "=", 5);
    println!("{}", s);
    const C: &str = concat!("const", "-", "str");
    println!("{}", C);
    println!("{}", stringify!());
    println!("[{}]", stringify!( ( ) [ ] { } ));
}
