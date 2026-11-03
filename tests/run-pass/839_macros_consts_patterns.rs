macro_rules! count { () => { 0usize }; ($h:tt $($t:tt)*) => { 1usize + count!($($t)*) }; }
macro_rules! make_enum {
    ($name:ident { $($var:ident = $val:expr),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq)]
        #[repr(i16)]
        enum $name { $($var = $val),+ }
        impl $name {
            const ALL: [$name; count!($($var)+)] = [$($name::$var),+];
            fn name(self) -> &'static str { match self { $($name::$var => stringify!($var)),+ } }
            fn from_i16(v: i16) -> Option<Self> { $(if v == $val { return Some($name::$var); })+ None }
        }
    };
}
make_enum!(Color { Red = -1, Green = 5, Blue = 300, });
macro_rules! hashmapish { ($($k:expr => $v:expr),* $(,)?) => {{ let mut m = std::collections::BTreeMap::new(); $( m.insert($k, $v); )* m }}; }
macro_rules! maxm { ($x:expr) => { $x }; ($x:expr, $($rest:expr),+) => {{ let a = $x; let b = maxm!($($rest),+); if a > b { a } else { b } }}; }
macro_rules! gen_struct {
    ($sname:ident, $($field:ident : $ty:ty = $def:expr),*) => {
        #[derive(Debug)] struct $sname { $($field: $ty),* }
        impl Default for $sname { fn default() -> Self { $sname { $($field: $def),* } } }
        impl $sname { fn fields() -> Vec<&'static str> { vec![$(stringify!($field)),*] } }
    };
}
gen_struct!(Settings, width: u32 = 80, title: String = "untitled".to_string(), ratio: f32 = 1.5);
macro_rules! define_ops { ($($fname:ident => $op:tt),*) => { $(fn $fname(a: i64, b: i64) -> i64 { a $op b })* }; }
define_ops!(addf => +, subf => -, mulf => *, shlf => <<);
macro_rules! make_macro { ($n:ident, $v:expr) => { macro_rules! $n { () => { $v * 2 } } }; }
make_macro!(twice_ten, 10);
macro_rules! matrix { ($([$($x:expr),*]),*) => { vec![$(vec![$($x),*]),*] }; }
macro_rules! my_vec { ($elem:expr; $n:expr) => { std::vec::from_elem($elem, $n) }; ($($x:expr),*) => { <[_]>::into_vec(Box::new([$($x),*])) }; }
macro_rules! tuple_sum { ($($i:tt),*) => { |t: (i32, i32, i32)| 0 $(+ t.$i)* }; }
macro_rules! nested_rep { ($($outer:ident: [$($inner:expr),*]);*) => { vec![$((stringify!($outer), 0 $(+ $inner)*)),*] }; }
const fn fib(n: u32) -> u64 { let mut a = 0; let mut b = 1; let mut i = 0; while i < n { let t = a + b; a = b; b = t; i += 1; } a }
const FIBS: [u64; 10] = { let mut arr = [0; 10]; let mut i = 0; while i < 10 { arr[i] = fib(i as u32 * 5); i += 1; } arr };
const LIMIT: i32 = 10; const LOW: i32 = -LIMIT;
#[derive(Debug)] struct P { x: i32, y: i32 }
static GREETING: &str = concat!("v", stringify!(1.2), "-", 3);
fn classify(p: &P) -> String {
    match p {
        P { x: 0, y: 0 } => "origin".into(),
        whole @ P { x: LOW..=-1, .. } => format!("left {:?}", whole),
        P { x: x @ (1 | 2 | 3), y: y @ 0..=LIMIT } => format!("small x={} y={}", x, y),
        P { x, y } if x == y => format!("diag {}", x),
        &P { x: xx @ 100.., y: _ } => format!("far {}", xx),
        _ => "other".into(),
    }
}
fn main() {
    println!("{:?} {} {:?} {:?}", Color::ALL, Color::Blue.name(), Color::from_i16(5), Color::from_i16(6));
    println!("{} {} {}", Color::Red as i16, Color::Blue as i16 as u8, Color::Green as i32 * 2);
    println!("{} {}", count!(a b c d e), count!());
    println!("{:?}", hashmapish!{"b" => 2, "a" => 1,});
    println!("{} {}", maxm!(3), maxm!(1, 9, 4, 7));
    println!("{:?} {:?}", Settings::default(), Settings::fields());
    println!("{} {} {} {}", addf(2, 3), subf(2, 3), mulf(2, 3), shlf(2, 3));
    println!("{}", twice_ten!());
    println!("{:?}", matrix!([1, 2], [3, 4], []));
    println!("{:?} {:?}", my_vec![7u8; 3], my_vec![1, 2, 3]);
    println!("{}", tuple_sum!(0, 1, 2)((10, 20, 30)));
    println!("{:?}", nested_rep!(a: [1, 2]; b: []; c: [5]));
    println!("{:?} {}", FIBS, fib(90));
    println!("{}", GREETING);
    for p in [P { x: 0, y: 0 }, P { x: -5, y: 1 }, P { x: -11, y: 1 }, P { x: 2, y: 10 }, P { x: 2, y: 11 }, P { x: 7, y: 7 }, P { x: 100, y: -1 }, P { x: 50, y: 0 }] {
        println!("{}", classify(&p));
    }
    println!("{} {}", line!(), module_path!());
    println!("{} {}", cfg!(debug_assertions), cfg!(target_os = "linux"));
    let v: Vec<Option<i32>> = vec![1, 2].into_iter().map(Some).collect();
    let w: Vec<P> = [(1, 2), (3, 4)].into_iter().map(|(x, y)| P { x, y }).collect();
    #[derive(Debug)] struct W(i32);
    let ws: Vec<W> = (1..4).map(W).collect();
    println!("{:?} {:?} {:?}", v, w, ws);
}
