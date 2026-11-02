macro_rules! square {
    ($x:expr) => {
        $x * $x
    };
}

macro_rules! max_of {
    ($x:expr) => { $x };
    ($x:expr, $($rest:expr),+) => {
        { let a = $x; let b = max_of!($($rest),+); if a > b { a } else { b } }
    };
}

macro_rules! my_vec {
    () => { Vec::new() };
    ($($x:expr),+ $(,)?) => {
        {
            let mut v = Vec::new();
            $( v.push($x); )+
            v
        }
    };
}

macro_rules! hashmap {
    ($($k:expr => $v:expr),* $(,)?) => {{
        let mut m = std::collections::BTreeMap::new();
        $( m.insert($k, $v); )*
        m
    }};
}

macro_rules! point_struct {
    ($name:ident, $($field:ident),+) => {
        #[derive(Debug, Default)]
        struct $name {
            $( $field: f64, )+
        }
        impl $name {
            fn norm(&self) -> f64 {
                (0.0 $( + self.$field * self.$field )+).sqrt()
            }
        }
    };
}

macro_rules! count {
    () => { 0usize };
    ($head:tt $($tail:tt)*) => { 1usize + count!($($tail)*) };
}

macro_rules! show {
    ($($e:expr),*) => {
        $( println!("{} = {:?}", stringify!($e), $e); )*
    };
}

macro_rules! op {
    (add $a:expr, $b:expr) => { $a + $b };
    (mul $a:expr, $b:expr) => { $a * $b };
    (neg $a:expr) => { -$a };
}

macro_rules! make_fn {
    ($fname:ident, $val:expr) => {
        fn $fname() -> i32 {
            $val
        }
    };
}

point_struct!(P3, x, y, z);
make_fn!(forty_two, 42);
make_fn!(seven, 3 + 4);

fn main() {
    println!("square(7) = {}", square!(7));
    println!("square(2+3) = {}", square!(2 + 3));
    println!("max_of = {}", max_of!(3, 9, 2, 7));
    println!("max_of single = {}", max_of!(5));
    let v: Vec<i32> = my_vec![1, 2, 3,];
    let e: Vec<i32> = my_vec![];
    println!("my_vec = {:?} {:?}", v, e);
    let m = hashmap! { "one" => 1, "two" => 2, "three" => 3 };
    println!("map = {:?}", m);
    let p = P3 { x: 1.0, y: 2.0, z: 2.0 };
    println!("{:?} norm = {}", p, p.norm());
    println!("default = {:?}", P3::default());
    println!("count = {}", count!(a b c d e));
    let x = 5;
    let name = "macro";
    show!(x, name, x * 2 + 1, vec![x; 3]);
    println!("ops = {} {} {}", op!(add 2, 3), op!(mul 4, 5), op!(neg 6));
    println!("fns = {} {}", forty_two(), seven());
    println!("stringify = {}", stringify!(a + b * c));
    println!("concat = {}", concat!("a", 1, "b", 2.5, true));
    println!("line is positive: {}", line!() > 0);
    let cond = true;
    assert!(cond, "should hold");
    assert_eq!(square!(3), 9);
    assert_ne!(square!(3), 10);
    debug_assert!(1 < 2);
    println!("assertions passed");
}
