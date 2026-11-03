macro_rules! show {
    ($e:expr) => { println!("{:<28} => {:?}", stringify!($e), $e) };
}

macro_rules! const_str {
    ($name:ident = $($part:expr),+) => { const $name: &str = concat!($($part),+); };
}
const_str!(GREETING = "hello", ", ", "world", '!', 1, 2.5, true);

macro_rules! version {
    ($maj:literal . $min:literal . $patch:literal) => {
        concat!("v", stringify!($maj), ".", stringify!($min), ".", stringify!($patch))
    };
}

macro_rules! field_path {
    ($($seg:ident).+) => { stringify!($($seg).+) };
}

macro_rules! type_in_pos {
    ($t:ty) => { <$t>::default() };
}

macro_rules! pat_in_pos {
    ($p:pat) => { |v: Option<i32>| matches!(v, $p) };
}

macro_rules! arr_type {
    ($t:ty; $n:expr) => { [$t; $n] };
}

macro_rules! line_info {
    () => { (line!() > 0, column!() > 0, file!().ends_with(".rs"), module_path!()) };
}

macro_rules! stmt_frag {
    ($s:stmt) => {{ $s; stringify!($s) }};
}

fn main() {
    println!("{}", stmt_frag!(let _unused: u8 = 5));
    println!("{}", stmt_frag!(let _w = 1 + 2));
    show!(1 + 2 * 3);
    show!(vec![1, 2, 3].len());
    show!("a".to_owned() + "b");
    show!(if true { 1 } else { 2 });
    show!([1u8; 3]);
    show!((1, 'c', "s"));
    println!("{}", GREETING);
    println!("{}", version!(1 . 72 . 0));
    println!("{}", field_path!(a.b.c));
    println!("{}", stringify!(struct Foo<'a, T: Clone> where T: Copy { x: &'a T }));
    println!("{}", stringify!(a+b - c));
    println!("{}", stringify!(x => y, z));
    println!("{}", stringify!(#[derive(Debug)] fn f() -> u8 { 0 }));
    println!("{}", concat!("x", 10, "y", -3, 'z'));
    let d: u16 = type_in_pos!(u16);
    let s: String = type_in_pos!(String);
    println!("{:?} {:?}", d, s);
    let check = pat_in_pos!(Some(1..=5) | None);
    println!("{} {} {}", check(Some(3)), check(None), check(Some(9)));
    let a: arr_type!(u8; 4) = [1, 2, 3, 4];
    println!("{:?}", a);
    let (l, c, f, m) = line_info!();
    println!("{} {} {} {}", l, c, f, m);
    println!("{}", stringify!(-1));
    println!("{}", stringify!(r"raw" b"bytes" 'x' 1_000u32 0xFFi64 1e10));
}
