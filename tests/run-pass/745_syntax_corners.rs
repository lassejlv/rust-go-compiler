// Syntax corners: labeled blocks with break-value, items and impls nested in
// function bodies, raw identifiers, raw/byte strings, builtin macros
// (concat!, stringify!, line!, matches!, cfg!), Self constructors/patterns,
// nested modules with super/self paths, compound assignment operators on
// custom types, and closures over shadowed variables.
use std::ops::{AddAssign, BitOr, BitOrAssign, Not, ShlAssign, SubAssign};

mod outer {
    pub mod inner {
        pub fn f() -> &'static str { super::helper() }
        pub(crate) fn g() -> String { format!("{}+{}", self::h(), crate::top_level()) }
        fn h() -> &'static str { "inner::h" }
    }
    fn helper() -> &'static str { "outer::helper" }
    pub use self::inner::f as reexported;
}
fn top_level() -> &'static str { "crate::top" }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Flags(u8);
impl Flags {
    const A: Flags = Self(1);
    const B: Flags = Self(2);
    const C: Flags = Self(4);
    fn has(self, o: Flags) -> bool { self.0 & o.0 == o.0 }
    fn names(self) -> Vec<&'static str> {
        [(Self::A, "A"), (Self::B, "B"), (Self::C, "C")].iter().filter(|(f, _)| self.has(*f)).map(|(_, n)| *n).collect()
    }
}
impl BitOr for Flags { type Output = Self; fn bitor(self, o: Self) -> Self { Self(self.0 | o.0) } }
impl BitOrAssign for Flags { fn bitor_assign(&mut self, o: Self) { self.0 |= o.0; } }
impl Not for Flags { type Output = Self; fn not(self) -> Self { Self(!self.0 & 7) } }

#[derive(Debug, Default)]
struct Acc { total: i64, log: Vec<String> }
impl AddAssign<i64> for Acc { fn add_assign(&mut self, v: i64) { self.total += v; self.log.push(format!("+{}", v)); } }
impl<'a> AddAssign<&'a str> for Acc { fn add_assign(&mut self, s: &str) { self.log.push(s.to_string()); } }
impl SubAssign for Acc { fn sub_assign(&mut self, o: Acc) { self.total -= o.total; self.log.extend(o.log.into_iter().map(|s| format!("-{}", s))); } }
impl ShlAssign<u32> for Acc { fn shl_assign(&mut self, n: u32) { self.total <<= n; } }

#[derive(Debug)]
enum Shape { Circle { r: u32 }, Rect(u32, u32), Empty }
impl Shape {
    fn new_square(s: u32) -> Self { Self::Rect(s, s) }
    fn area(&self) -> u32 {
        match self { Self::Circle { r } => 3 * r * r, Self::Rect(w, h) => w * h, Self::Empty => 0 }
    }
}

fn r#match(r#type: &str) -> String { format!("raw ident fn got {}", r#type) }

fn main() {
    let v = [3, 7, 12, 5];
    let found = 'search: {
        for (i, &x) in v.iter().enumerate() {
            if x > 10 { break 'search Some(i); }
        }
        None
    };
    println!("labeled block: {:?}", found);
    let mut grid_hits = vec![];
    'rows: for r in 0..4 {
        'cols: for c in 0..4 {
            if c > r { continue 'rows; }
            if (r + c) % 3 == 2 { continue 'cols; }
            if r == 3 && c == 2 { break 'rows; }
            grid_hits.push((r, c));
        }
    }
    println!("labeled loops: {:?}", grid_hits);
    let val = loop { break 'x'; };
    println!("loop break value {:?}", val);

    fn nested_helper(x: u32) -> u32 {
        struct Local(u32);
        impl Local { fn twice(&self) -> u32 { self.0 * 2 } }
        Local(x).twice() + 1
    }
    println!("nested fn+struct+impl: {}", nested_helper(20));
    {
        use std::collections::BTreeSet as Set;
        let s: Set<_> = "hello".chars().collect();
        println!("scoped use: {:?}", s);
    }
    println!("{} | {} | {}", outer::inner::f(), outer::inner::g(), outer::reexported());

    let mut fl = Flags::A | Flags::C;
    println!("flags {:?} names {:?} not {:?}", fl, fl.names(), (!fl).names());
    fl |= Flags::B;
    println!("after |= {:?} has B {}", fl.names(), fl.has(Flags::B));

    let mut acc = Acc::default();
    acc += 5;
    acc += "note";
    acc += 10;
    let mut other = Acc::default();
    other += 3;
    acc -= other;
    acc <<= 2;
    println!("acc {:?}", acc);

    for s in [Shape::Circle { r: 2 }, Shape::new_square(4), Shape::Empty, Shape::Rect(2, 9)] {
        print!("{:?}={} ", s, s.area());
    }
    println!();
    println!("{}", r#match("r#type"));
    let r#loop = 5;
    println!("raw local {}", r#loop * 2);

    let raw = r#"He said "hi" \n not escaped"#;
    let raw2 = r##"contains "# inside"##;
    let bytes = b"bytes\x00\xff";
    let bstr = br"raw\bytes";
    println!("{} | {} | {:?} | {:?} | {}", raw, raw2, bytes, bstr, bstr.len());
    println!("escapes: {:?} {} {}", "\u{1F600}\t\\\'\"\0", '\x41', "multi\
              line continuation");
    println!("{}", concat!("a", 1, 2.5, true, 'c', -3));
    println!("{}", stringify!(x + y * (z - 1) => fn foo<T>() {}));
    let here = line!();
    println!("line! is {} debug_assertions={}", here, cfg!(debug_assertions));
    println!("{}", matches!(Shape::Rect(1, 2), Shape::Rect(a, _) if a > 0));

    let x = 1;
    let add_x = |y| x + y;
    let x = 100;
    println!("closure captured pre-shadow x: {} (x now {})", add_x(1), x);
    let x = x.to_string() + "!";
    println!("shadowed type change: {}", x);
    let t = (1, (2.5, "s"), [4, 5]);
    let (a, (b, c), [d, e]) = t;
    println!("destructure {} {} {} {} {} / t.1.1={}", a, b, c, d, e, t.1 .1);
    let unit = ();
    let ((), n) = (unit, 3);
    println!("unit pattern {}", n);
    #[allow(unused_parens)]
    let p = ((((7))));
    let neg = -(-p) - -1;
    println!("parens {} {} {}", p, neg, !-1i32);
    let arr2: [[char; 2]; 2] = [['a', 'b'], ['c', 'd']];
    println!("{}", arr2.iter().flatten().rev().collect::<String>());
}
