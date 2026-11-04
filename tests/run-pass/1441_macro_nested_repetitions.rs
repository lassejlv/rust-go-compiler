// nested repetitions, separators, optional fragments
macro_rules! matrix {
    ($( $( $x:expr ),+ );*) => { vec![ $( vec![ $( $x ),* ] ),* ] };
}

macro_rules! table {
    ($( $row:ident => [ $( $col:ident = $v:expr ),* ] )*) => {{
        let mut out = Vec::new();
        $(
            $( out.push(format!("{}.{}={}", stringify!($row), stringify!($col), $v)); )*
        )*
        out
    }};
}

macro_rules! opt_default {
    ($name:ident $(: $t:ty)? $(= $e:expr)?) => {{
        #[allow(unused_mut, unused_assignments)]
        let mut $name $(: $t)? = Default::default();
        $( $name = $e; )?
        $name
    }};
}

macro_rules! sum_pairs {
    ($( ($a:expr, $b:expr) ),+) => { 0 $( + $a * $b )+ };
}

macro_rules! enum_with_strs {
    ($vis:vis enum $name:ident { $( $variant:ident $( = $disc:expr )? ),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq)]
        $vis enum $name { $( $variant $( = $disc )? ),+ }
        impl $name {
            $vis const ALL: &'static [$name] = &[$( $name::$variant ),+];
            $vis fn as_str(&self) -> &'static str {
                match self { $( $name::$variant => stringify!($variant) ),+ }
            }
        }
    };
}

enum_with_strs!(pub enum Color { Red = 1, Green = 4, Blue });
enum_with_strs!(enum Dir { N, E, S, W, });

macro_rules! zip_names {
    ($($a:ident)* ; $($b:literal)*) => { [ $( (stringify!($a), $b) ),* ] };
}

macro_rules! sep_variety {
    ($($x:expr)=>*) => { [$($x),*] };
}

fn main() {
    let m = matrix![1, 2, 3; 4, 5, 6; 7, 8, 9];
    println!("{:?}", m);
    let e: Vec<Vec<i32>> = matrix![];
    println!("{:?}", e);
    for line in table!(a => [x = 1, y = 2] b => [] c => [z = "q"]) { println!("{}", line); }
    let v = opt_default!(v: i64 = 42);
    let w = opt_default!(w: String);
    let z: u8 = opt_default!(z = 7);
    println!("{} {:?} {}", v, w, z);
    println!("{}", sum_pairs!((1, 2), (3, 4), (5, 6)));
    for c in Color::ALL { println!("{} {}", c.as_str(), *c as i32); }
    println!("{:?}", Dir::ALL.iter().map(|d| d.as_str()).collect::<Vec<_>>());
    println!("{:?}", zip_names!(p q r ; 1 2 3));
    println!("{:?}", sep_variety!(1 => 2 => 3));
}
