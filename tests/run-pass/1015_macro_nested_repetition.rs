use std::collections::BTreeMap;

macro_rules! matrix {
    ($([$($x:expr),*]),* $(,)?) => { vec![$(vec![$($x),*]),*] };
}

macro_rules! btree {
    ($($k:expr => [$($v:expr),*]),* $(,)?) => {{
        let mut m = BTreeMap::new();
        $( m.insert($k, vec![$($v),*]); )*
        m
    }};
}

macro_rules! cross {
    ($($a:ident),* ; $bs:tt) => {
        vec![$( cross!(@inner $a ; $bs) ),*]
    };
    (@inner $a:ident ; [$($b:literal),*]) => {
        vec![$(format!("{}{}", stringify!($a), $b)),*]
    };
}

macro_rules! impl_trait_for {
    ($tr:ident $body:tt for $($t:ty),+) => {
        $( impl_trait_for!(@one $tr $t $body); )+
    };
    (@one $tr:ident $t:ty { $($m:ident -> $r:expr);* }) => {
        impl $tr for $t {
            $( fn $m(&self) -> String { format!("{}:{}", stringify!($t), $r) } )*
        }
    };
}

trait Describe { fn short(&self) -> String; fn long(&self) -> String; }
impl_trait_for!(Describe { short -> "s"; long -> "looong" } for i32, String, Vec<u8>);

macro_rules! sum_groups {
    ($($($n:literal)+);*) => { [$(0 $(+ $n)+),*] };
}

macro_rules! optional {
    ($name:ident $(: $t:ty)? $(= $def:expr)?) => {{
        let v $(: $t)? = optional!(@def $($def)?);
        (stringify!($name), v)
    }};
    (@def) => { Default::default() };
    (@def $e:expr) => { $e };
}

macro_rules! sep_variety {
    ($($x:expr);+ $(;)?) => { [$($x),+] };
}

macro_rules! zip_pairs {
    (($($a:expr),*) ($($b:expr),*)) => { [$(($a, $b)),*] };
}

fn main() {
    let m = matrix![[1, 2, 3], [4, 5, 6], [7, 8, 9],];
    println!("{:?}", m);
    let t: Vec<Vec<i32>> = (0..3).map(|c| m.iter().map(|r| r[c]).collect()).collect();
    println!("{:?}", t);
    let b = btree! { "b" => [2, 3], "a" => [1], "c" => [] };
    println!("{:?}", b);
    println!("{:?}", cross!(x, y ; [1, 2, 3]));
    println!("{} {}", 5i32.short(), String::from("q").long());
    println!("{}", vec![1u8].long());
    println!("{:?}", sum_groups!(1 2 3; 10 20; 7));
    let a: (&str, u8) = optional!(a: u8);
    let b = optional!(b = "given");
    let c = optional!(c: f32 = 1.5);
    println!("{:?} {:?} {:?}", a, b, c);
    println!("{:?}", sep_variety!(1; 2; 3;));
    println!("{:?}", zip_pairs!((1, 2, 3) ("a", "b", "c")));
}
