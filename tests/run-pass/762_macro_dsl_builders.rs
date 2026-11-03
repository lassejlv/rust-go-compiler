// macro_rules DSLs: an html!-like tree builder (tt muncher with nested
// braces and attributes), a struct+builder generator with defaults and
// field docs, a state-machine generator with transition tables, counting
// with recursive macros, and macros generating trait impls for many types.
use std::fmt::Write;

#[derive(Debug)]
enum El { Tag(String, Vec<(String, String)>, Vec<El>), Text(String) }

impl El {
    fn render(&self, out: &mut String, depth: usize) {
        match self {
            El::Text(t) => { let _ = writeln!(out, "{}{}", "  ".repeat(depth), t); }
            El::Tag(name, attrs, kids) => {
                let a: String = attrs.iter().map(|(k, v)| format!(" {}=\"{}\"", k, v)).collect();
                let _ = writeln!(out, "{}<{}{}>", "  ".repeat(depth), name, a);
                for k in kids { k.render(out, depth + 1); }
                let _ = writeln!(out, "{}</{}>", "  ".repeat(depth), name);
            }
        }
    }
}

macro_rules! html {
    ($tag:ident [ $($k:ident = $v:expr),* ] { $($body:tt)* }) => {
        El::Tag(stringify!($tag).to_string(), vec![$((stringify!($k).to_string(), $v.to_string())),*], html!(@kids [] $($body)*))
    };
    ($tag:ident { $($body:tt)* }) => { html!($tag [] { $($body)* }) };
    (@kids [$($done:expr),*]) => { vec![$($done),*] };
    (@kids [$($done:expr),*] $text:literal $($rest:tt)*) => { html!(@kids [$($done,)* El::Text($text.to_string())] $($rest)*) };
    (@kids [$($done:expr),*] ($e:expr) $($rest:tt)*) => { html!(@kids [$($done,)* El::Text(format!("{}", $e))] $($rest)*) };
    (@kids [$($done:expr),*] $tag:ident [ $($attrs:tt)* ] { $($inner:tt)* } $($rest:tt)*) => {
        html!(@kids [$($done,)* html!($tag [ $($attrs)* ] { $($inner)* })] $($rest)*)
    };
    (@kids [$($done:expr),*] $tag:ident { $($inner:tt)* } $($rest:tt)*) => {
        html!(@kids [$($done,)* html!($tag { $($inner)* })] $($rest)*)
    };
}

macro_rules! builder {
    ($name:ident => $bname:ident { $( $(#[doc = $doc:literal])? $field:ident : $ty:ty = $default:expr ),+ $(,)? }) => {
        #[derive(Debug, Clone, PartialEq)]
        struct $name { $($field: $ty),+ }
        #[derive(Default)]
        struct $bname { $($field: Option<$ty>),+ }
        impl $bname {
            $( fn $field(mut self, v: impl Into<$ty>) -> Self { self.$field = Some(v.into()); self } )+
            fn build(self) -> $name { $name { $($field: self.$field.unwrap_or_else(|| $default)),+ } }
        }
        impl $name {
            fn describe_fields() -> Vec<String> {
                vec![$(format!("{}: {}{}", stringify!($field), stringify!($ty), { let d: &str = concat!("" $(, " -- ", $doc)?); d })),+]
            }
        }
    };
}

builder!(Config => ConfigBuilder {
    #[doc = "host name"]
    host: String = "localhost".to_string(),
    port: u16 = 8080,
    #[doc = "worker threads"]
    workers: usize = 4,
    verbose: bool = false,
});

macro_rules! state_machine {
    ($name:ident { $($state:ident),+ } start $start:ident; $( $from:ident --$ev:ident--> $to:ident ;)+ ) => {
        #[derive(Debug, Clone, Copy, PartialEq)]
        enum $name { $($state),+ }
        impl $name {
            fn start() -> Self { $name::$start }
            fn on(self, ev: &str) -> Result<Self, String> {
                match (self, ev) {
                    $( ($name::$from, stringify!($ev)) => Ok($name::$to), )+
                    (s, e) => Err(format!("{:?} cannot handle {}", s, e)),
                }
            }
            fn transitions() -> usize { [$(stringify!($ev)),+].len() }
        }
    };
}

state_machine!(Door { Closed, Open, Locked } start Closed;
    Closed --open--> Open;
    Open --close--> Closed;
    Closed --lock--> Locked;
    Locked --unlock--> Closed;
);

macro_rules! count { () => { 0usize }; ($head:tt $($tail:tt)*) => { 1usize + count!($($tail)*) }; }

trait Describe { fn describe(&self) -> String; }
macro_rules! impl_describe {
    ($($t:ty => $fmt:literal),* $(,)?) => { $( impl Describe for $t { fn describe(&self) -> String { format!($fmt, self) } } )* };
}
impl_describe!(i32 => "int {}", f64 => "float {:.2}", &str => "str {:?}", bool => "bool {}", char => "char {:?}");

macro_rules! maxm {
    ($x:expr) => { $x };
    ($x:expr, $($rest:expr),+) => {{ let a = $x; let b = maxm!($($rest),+); if a > b { a } else { b } }};
}

macro_rules! hashmap_lit {
    ($($k:expr => $v:expr),* $(,)?) => {{ let mut m = std::collections::BTreeMap::new(); $( m.insert($k, $v); )* m }};
}

fn main() {
    let items = ["alpha", "beta"];
    let page = html!(div [class = "card", id = 7] {
        h1 { "Title" }
        p [style = "x"] { "some text" (items.len()) }
        ul { li { (items[0]) } li { (items[1]) } }
        "trailing"
    });
    let mut out = String::new();
    page.render(&mut out, 0);
    print!("{}", out);

    let c = ConfigBuilder::default().port(9000u16).verbose(true).build();
    let d = ConfigBuilder::default().host("example.org").workers(16usize).build();
    println!("{:?}\n{:?}", c, d);
    for f in Config::describe_fields() { println!("  {}", f); }

    let mut s = Door::start();
    for ev in ["open", "close", "lock", "open", "unlock", "open"] {
        match s.on(ev) { Ok(n) => { println!("{:?} --{}--> {:?}", s, ev, n); s = n; } Err(e) => println!("error: {}", e) }
    }
    println!("transitions {}", Door::transitions());
    println!("count {} {}", count!(a b c d e), count!());
    println!("{} | {} | {} | {} | {}", 5.describe(), 2.345.describe(), "q".describe(), true.describe(), 'z'.describe());
    println!("max {} {}", maxm!(3, 9, 2, 7), maxm!("a", "c", "b"));
    println!("{:?}", hashmap_lit!{"one" => 1, "two" => 2, "three" => 3,});
}
