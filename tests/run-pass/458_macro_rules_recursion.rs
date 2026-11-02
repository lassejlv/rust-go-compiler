// macro_rules with repetitions ($(...),* and $(...);*), nested repetitions, recursion (tt munching),
// internal rules (@inner), counting via recursion, trailing separators, literal/ident/expr/ty/pat/
// block fragment specifiers, and macros expanding to items, expressions, and statements.
macro_rules! count {
    () => { 0usize };
    ($head:tt $($tail:tt)*) => { 1usize + count!($($tail)*) };
}

macro_rules! max {
    ($x:expr) => { $x };
    ($x:expr, $($rest:expr),+ $(,)?) => {{
        let a = $x;
        let b = max!($($rest),+);
        if a > b { a } else { b }
    }};
}

macro_rules! hashmap {
    ($($k:expr => $v:expr),* $(,)?) => {{
        let mut m = std::collections::BTreeMap::new();
        $( m.insert($k, $v); )*
        m
    }};
}

macro_rules! calc {
    (@op $acc:expr ;) => { $acc };
    (@op $acc:expr ; + $n:literal $($rest:tt)*) => { calc!(@op ($acc + $n) ; $($rest)*) };
    (@op $acc:expr ; - $n:literal $($rest:tt)*) => { calc!(@op ($acc - $n) ; $($rest)*) };
    (@op $acc:expr ; * $n:literal $($rest:tt)*) => { calc!(@op ($acc * $n) ; $($rest)*) };
    ($first:literal $($rest:tt)*) => { calc!(@op $first ; $($rest)*) };
}

macro_rules! matrix {
    ($([$($x:expr),*]),*) => { vec![$(vec![$($x),*]),*] };
}

macro_rules! make_struct {
    ($name:ident { $($field:ident : $t:ty = $default:expr),* $(,)? }) => {
        #[derive(Debug)]
        struct $name { $($field: $t),* }
        impl $name {
            fn new() -> Self { $name { $($field: $default),* } }
            fn field_names() -> &'static [&'static str] { &[$(stringify!($field)),*] }
        }
    };
}

macro_rules! check_pat {
    ($v:expr, $p:pat) => { matches!($v, $p) };
}

macro_rules! repeat_block {
    ($n:expr, $body:block) => {
        for _ in 0..$n $body
    };
}

macro_rules! reverse_list {
    (@acc [$($out:expr),*]) => { vec![$($out),*] };
    (@acc [$($out:expr),*] $head:expr $(, $tail:expr)*) => { reverse_list!(@acc [$head $(, $out)*] $($tail),*) };
    ($($x:expr),*) => { reverse_list!(@acc [] $($x),*) };
}

macro_rules! nested {
    ($($outer:ident: [$($inner:literal)*]);* $(;)?) => {
        vec![$( (stringify!($outer), 0 $(+ $inner)*, count!($($inner)*)) ),*]
    };
}

make_struct!(Settings { width: u32 = 80, height: u32 = 24, title: String = String::from("term"), });

fn main() {
    println!("{} {} {}", count!(), count!(a), count!(a b c d e));
    println!("{}", max!(3));
    println!("{}", max!(3, 9, 2, 7,));
    println!("{}", max!("pear", "apple", "zoo"));
    let m = hashmap! { "one" => 1, "two" => 2, "three" => 3 };
    println!("{:?}", m);
    let empty: std::collections::BTreeMap<i32, i32> = hashmap! {};
    println!("{:?}", empty);
    println!("{}", calc!(2 + 3 * 4 - 1));
    println!("{}", calc!(10));
    let mx = matrix!([1, 2, 3], [4, 5, 6]);
    println!("{:?}", mx);
    let s = Settings::new();
    println!("{:?} {:?}", s, Settings::field_names());
    println!("{} {}", check_pat!(Some(5), Some(1..=9)), check_pat!('x', 'a' | 'b'));
    let mut n = 0;
    repeat_block!(4, { n += 3; });
    println!("{}", n);
    println!("{:?}", reverse_list!(1, 2, 3, 4));
    println!("{:?}", nested!(a: [1 2 3]; b: []; c: [10 20];));
    let v: Vec<String> = vec![stringify!(a + b * c), stringify!(fn f() {}), concat!("x", 1, 2.5, true)].into_iter().map(String::from).collect();
    println!("{:?}", v);
    println!("{}:{}", file!().ends_with(".rs"), line!() > 0);
}
