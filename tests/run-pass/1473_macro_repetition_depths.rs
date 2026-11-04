// outer-depth metavariables used inside inner repetitions, multi-char separators, triple nesting
macro_rules! cross {
    ($( $a:ident : [ $( $b:literal ),* ] );* $(;)?) => {
        vec![ $( $( (stringify!($a), $b) ),* ),* ]
    };
}
macro_rules! prefix_all {
    ($p:literal; $($s:literal)*) => { [ $( concat!($p, $s) ),* ] };
}
macro_rules! chain {
    ($($i:ident)->+) => { [$(stringify!($i)),+].join(" -> ") };
}
macro_rules! paths {
    ($($seg:ident)::*) => { <[&str]>::len(&[$(stringify!($seg)),*]) };
}
macro_rules! triple {
    ($( $x:ident { $( $y:ident [ $( $z:literal )* ] )* } )*) => {{
        let mut v: Vec<String> = Vec::new();
        $( $( $( v.push(format!("{}.{}.{}", stringify!($x), stringify!($y), $z)); )* )* )*
        v
    }};
}
macro_rules! struct_with_impls {
    ($name:ident; $( $tr:ident => $( $m:ident -> $r:expr ),* );*) => {
        struct $name;
        $(
            trait $tr { $( fn $m(&self) -> i32; )* }
            impl $tr for $name { $( fn $m(&self) -> i32 { $r } )* }
        )*
    };
}
struct_with_impls!(Obj; A => a1 -> 1, a2 -> 2; B => b1 -> 10);

macro_rules! fold_ops {
    ($init:expr $(, $op:tt $v:expr)*) => {{ let mut acc = $init; $( acc = acc $op $v; )* acc }};
}
macro_rules! sep_semi_opt {
    ($($e:expr);* $(;)?) => { vec![$($e),*] };
}

fn main() {
    println!("{:?}", cross!(x: [1, 2]; y: [7]; z: [3]));
    println!("{:?}", prefix_all!("pre-"; "a" "b" "c"));
    println!("{}", chain!(a -> b -> c));
    println!("{} {}", paths!(a::b::c), paths!());
    println!("{:?}", triple!(p { q [1 2] r [] } s { }));
    println!("{} {} {}", Obj.a1(), Obj.a2(), Obj.b1());
    println!("{}", fold_ops!(1, + 2, * 10, - 5, / 5));
    println!("{:?}", sep_semi_opt!(1; 2; 3;));
    println!("{:?}", sep_semi_opt!(4));
}
