// Nested macro repetitions: $( $( ... )* )*, separators, multiple repeated groups in lockstep,
// generating structs with methods per field, test tables and matrices.

macro_rules! matrix {
    ( $( [ $( $x:expr ),* ] ),* $(,)? ) => {
        vec![ $( vec![ $( $x ),* ] ),* ]
    };
}

macro_rules! sum_rows {
    ( $( [ $( $x:expr ),* ] )* ) => {
        [ $( 0 $( + $x )* ),* ]
    };
}

// Several structs, each with several fields, each field gets a getter and a setter.
macro_rules! records {
    ( $( $name:ident { $( $field:ident : $ty:ty ),* $(,)? } )* ) => {
        $(
            #[derive(Debug, Default, Clone)]
            struct $name { $( $field: $ty ),* }

            #[allow(dead_code)]
            impl $name {
                const FIELDS: &'static [&'static str] = &[ $( stringify!($field) ),* ];
                fn describe(&self) -> String {
                    let mut parts: Vec<String> = Vec::new();
                    $( parts.push(format!("{}={:?}", stringify!($field), self.$field)); )*
                    format!("{}{{{}}}", stringify!($name), parts.join(", "))
                }
            }
        )*

        fn all_schemas() -> Vec<(&'static str, Vec<(&'static str, &'static str)>)> {
            vec![ $( (stringify!($name), vec![ $( (stringify!($field), stringify!($ty)) ),* ]) ),* ]
        }
    };
}

records! {
    User { id: u32, name: String, admin: bool }
    Post { id: u32, author: u32, title: String, tags: Vec<String> }
    Empty {}
}

// Lockstep repetition of two groups: names and values must have equal length
macro_rules! zip_pairs {
    ( [ $( $k:ident ),* ] [ $( $v:expr ),* ] ) => {
        vec![ $( (stringify!($k), $v) ),* ]
    };
}

// Table-driven checks: each case with a list of inputs
macro_rules! cases {
    ( $( $fname:ident ( $( $arg:expr ),* ) => $expected:expr ; )* ) => {{
        let mut results = Vec::new();
        $(
            let got = $fname( $( $arg ),* );
            results.push(format!("{}({}) = {} [{}]",
                stringify!($fname),
                { let v: Vec<String> = vec![ $( format!("{:?}", $arg) ),* ]; v.join(", ") },
                got,
                if got == $expected { "ok" } else { "FAIL" }));
        )*
        results
    }};
}

fn add(a: i32, b: i32) -> i32 {
    a + b
}
fn clamp3(x: i32, lo: i32, hi: i32) -> i32 {
    x.clamp(lo, hi)
}
fn neg(x: i32) -> i32 {
    -x
}
fn answer() -> i32 {
    42
}

// Nested repetition with separators of different kinds and a trailing group
macro_rules! enum_with_groups {
    ( $ename:ident : $( $group:ident => [ $( $v:ident ),+ ] );+ $(;)? ) => {
        #[derive(Debug, Clone, Copy, PartialEq)]
        enum $ename { $( $( $v ),+ ),+ }
        impl $ename {
            fn group(&self) -> &'static str {
                match self {
                    $( $( $ename::$v )|+ => stringify!($group), )+
                }
            }
            fn all() -> Vec<$ename> { vec![ $( $( $ename::$v ),+ ),+ ] }
        }
    };
}

enum_with_groups! {
    Key:
    letters => [A, B, C];
    digits => [One, Two];
    special => [Enter];
}

fn main() {
    let m = matrix![[1, 2, 3], [4, 5, 6], [7, 8, 9]];
    println!("{:?}", m);
    let ragged: Vec<Vec<i32>> = matrix![[1], [], [2, 3]];
    println!("{:?}", ragged);
    println!("{:?}", sum_rows!([1, 2, 3] [10, 20] [] [100]));

    let u = User { id: 1, name: "ada".into(), admin: true };
    let p = Post { id: 9, author: 1, title: "Hello".into(), tags: vec!["intro".into()] };
    println!("{}", u.describe());
    println!("{}", p.describe());
    println!("{}", Empty::default().describe());
    println!("{:?} {:?} {:?}", User::FIELDS, Post::FIELDS, Empty::FIELDS);
    for (name, fields) in all_schemas() {
        let fs: Vec<String> = fields.iter().map(|(f, t)| format!("{}: {}", f, t)).collect();
        println!("schema {} ({})", name, fs.join("; "));
    }

    println!("{:?}", zip_pairs!([width, height, depth] [10, 20, 30 + 1]));

    for line in cases! {
        add(1, 2) => 3;
        add(-5, 5) => 0;
        clamp3(15, 0, 10) => 10;
        neg(7) => -7;
        answer() => 41;
    } {
        println!("{}", line);
    }

    for k in Key::all() {
        print!("{:?}:{} ", k, k.group());
    }
    println!();
}
