// macro_rules: tt munchers, internal rules, counting, nested repetitions, recursion, $crate, literal matching
macro_rules! count {
    () => { 0usize };
    ($head:tt $($tail:tt)*) => { 1usize + count!($($tail)*) };
}

macro_rules! rpn {
    (@go [$($stack:expr),*]) => { [$($stack),*] };
    (@go [$a:expr, $b:expr $(, $rest:expr)*] + $($tail:tt)*) => { rpn!(@go [$b + $a $(, $rest)*] $($tail)*) };
    (@go [$a:expr, $b:expr $(, $rest:expr)*] * $($tail:tt)*) => { rpn!(@go [$b * $a $(, $rest)*] $($tail)*) };
    (@go [$a:expr, $b:expr $(, $rest:expr)*] - $($tail:tt)*) => { rpn!(@go [$b - $a $(, $rest)*] $($tail)*) };
    (@go [$($stack:expr),*] $num:literal $($tail:tt)*) => { rpn!(@go [$num $(, $stack)*] $($tail)*) };
    ($($tokens:tt)*) => { rpn!(@go [] $($tokens)*) };
}

macro_rules! hashmap {
    (@single $($x:tt)*) => (());
    (@count $($rest:expr),*) => (<[()]>::len(&[$(hashmap!(@single $rest)),*]));
    ($($key:expr => $value:expr),* $(,)?) => {{
        let cap = hashmap!(@count $($key),*);
        let mut map = std::collections::BTreeMap::new();
        let _ = cap;
        $( map.insert($key, $value); )*
        map
    }};
}

macro_rules! matrix {
    ($([$($x:expr),*]),*) => {
        vec![$(vec![$($x),*]),*]
    };
}

macro_rules! html {
    ($tag:ident { $($inner:tt)* }) => {
        format!("<{}>{}</{}>", stringify!($tag), html!(@children $($inner)*), stringify!($tag))
    };
    ($text:literal) => { $text.to_string() };
    (@children) => { String::new() };
    (@children $tag:ident { $($inner:tt)* } $($rest:tt)*) => {
        html!($tag { $($inner)* }) + &html!(@children $($rest)*)
    };
    (@children $text:literal $($rest:tt)*) => {
        html!($text) + &html!(@children $($rest)*)
    };
}

macro_rules! enum_str {
    ($(#[$m:meta])* enum $name:ident { $($variant:ident = $val:expr),+ $(,)? }) => {
        $(#[$m])*
        enum $name { $($variant = $val),+ }
        impl $name {
            fn name(&self) -> &'static str {
                match self { $($name::$variant => stringify!($variant)),+ }
            }
            fn all() -> Vec<$name> { vec![$($name::$variant),+] }
            const COUNT: usize = count!($($variant)+);
        }
    };
}

enum_str! {
    #[derive(Debug, Clone, Copy, PartialEq)]
    enum Color { Red = 1, Green = 2, Blue = 4, Alpha = 8 }
}

macro_rules! max {
    ($x:expr) => { $x };
    ($x:expr, $($rest:expr),+) => {{ let a = $x; let b = max!($($rest),+); if a > b { a } else { b } }};
}

macro_rules! op_kind {
    (+) => { "plus" };
    (-) => { "minus" };
    ($other:tt) => { concat!("other:", stringify!($other)) };
}

macro_rules! fields {
    ($s:ident; $($f:ident),*) => {
        vec![$( (stringify!($f), format!("{:?}", $s.$f)) ),*]
    };
}

macro_rules! nested_rep {
    ($( $name:ident : [ $( $v:expr ),* ] );* $(;)?) => {
        vec![ $( (stringify!($name), 0 $( + $v )*) ),* ]
    };
}

struct Pt {
    x: i32,
    y: i32,
    label: &'static str,
}

fn main() {
    println!("{}", count!(a b c d e));
    println!("{}", count!());
    println!("{:?}", rpn!(3 4 + 2 *));
    println!("{:?}", rpn!(10 2 3 * -));
    println!("{:?}", rpn!(1 2 3));
    let m = hashmap! { "one" => 1, "two" => 2, "three" => 3, };
    println!("{:?} {}", m, hashmap!(@count 1, 2, 3, 4));
    let mx = matrix!([1, 2, 3], [4, 5, 6]);
    println!("{:?}", mx);
    println!("{}", html!(div { p { "hello" } span { "a" "b" } }));
    for c in Color::all() {
        print!("{}={} ", c.name(), c as i32);
    }
    println!("count={}", Color::COUNT);
    println!("{}", max!(3, 9, 2, 7));
    println!("{}", max!("pear", "apple", "zoo"));
    println!("{} {} {} {}", op_kind!(+), op_kind!(-), op_kind!(*), op_kind!(foo));
    let p = Pt { x: 1, y: -2, label: "origin-ish" };
    println!("{:?}", fields!(p; x, y, label));
    println!("{:?}", nested_rep!(a: [1, 2, 3]; b: []; c: [10]));
    let v: Vec<u8> = vec![1; count!(x x x)];
    println!("{:?}", v);
}
