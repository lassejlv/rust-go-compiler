// derived PartialOrd/Ord on enums & structs, Hash (incl. exact DefaultHasher values), Default derive with #[default]
use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeSet, HashSet};
use std::hash::{BuildHasher, BuildHasherDefault, Hash, Hasher};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Suit {
    Clubs,
    Diamonds,
    Hearts,
    Spades,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Shape {
    Point,
    Circle(u32),
    Rect { w: u32, h: u32 },
    Poly(Vec<(i32, i32)>),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
struct Version {
    major: u32,
    minor: u32,
    patch: u32,
    pre: Option<String>,
}

#[derive(Debug, PartialEq, PartialOrd)]
struct Measure(f64, &'static str);

#[derive(Debug, Default, Clone, PartialEq)]
enum Mode {
    Fast,
    #[default]
    Balanced,
    Slow(u8),
}

#[derive(Debug, Default)]
struct Settings {
    mode: Mode,
    level: i8,
    name: String,
    tags: Vec<&'static str>,
    ratio: f32,
    flag: Option<bool>,
    pair: (u8, char),
}

fn h<T: Hash + ?Sized>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

fn main() {
    let mut suits = vec![Suit::Spades, Suit::Clubs, Suit::Hearts, Suit::Diamonds];
    suits.sort();
    println!("{:?}", suits);
    println!("{} {:?}", Suit::Hearts > Suit::Diamonds, Suit::Clubs.cmp(&Suit::Spades));
    println!("{:?} {:?}", suits.iter().max(), Suit::Hearts.max(Suit::Clubs));
    let mut shapes = vec![
        Shape::Rect { w: 2, h: 1 },
        Shape::Circle(5),
        Shape::Poly(vec![(0, 0), (1, 1)]),
        Shape::Point,
        Shape::Rect { w: 1, h: 9 },
        Shape::Circle(3),
        Shape::Poly(vec![(0, 0)]),
        Shape::Poly(vec![(-1, 5)]),
    ];
    shapes.sort();
    for s in &shapes {
        println!("{:?}", s);
    }
    let mut vs = vec![
        Version { major: 1, minor: 2, patch: 3, pre: None },
        Version { major: 1, minor: 2, patch: 3, pre: Some("beta".into()) },
        Version { major: 1, minor: 10, patch: 0, pre: None },
        Version { major: 0, minor: 99, patch: 99, pre: Some("alpha".into()) },
        Version::default(),
    ];
    vs.sort_by(|a, b| b.cmp(a));
    for v in &vs {
        println!("{}.{}.{} {:?}", v.major, v.minor, v.patch, v.pre);
    }
    println!("{:?}", Measure(1.0, "a").partial_cmp(&Measure(f64::NAN, "a")));
    println!("{:?}", Measure(1.0, "b").partial_cmp(&Measure(1.0, "a")));
    println!("{}", Measure(2.0, "a") > Measure(1.0, "z"));
    println!("{:?}", (1, "b", 3.5) < (1, "b", 4.0));
    println!("{:?}", [1, 2, 3][..] < [1, 3][..]);
    println!("{:?}", "Zebra" < "apple");
    println!("{:?}", Some(0) > None);
    println!("{:?}", Ok::<i32, i32>(5).cmp(&Err(0)));
    let set: HashSet<Shape> = shapes.iter().cloned().chain(std::iter::once(Shape::Circle(5))).collect();
    println!("{}", set.len());
    let bset: BTreeSet<&Version> = vs.iter().collect();
    println!("{:?}", bset.iter().next().map(|v| v.minor));
    println!("{}", h(&Suit::Hearts) == h(&Suit::Hearts));
    println!("{}", h(&(1u8, 2u8)) == h(&(1u8, 2u8)));
    println!("{}", h(&"ab") == h(&String::from("ab")));
    println!("{}", h(&("a", "bc")) != h(&("ab", "c")));
    println!("{}", h(&vec![1u32, 2]) == h(&[1u32, 2][..]));
    println!("{:016x}", h(&0u64));
    println!("{:016x}", h(&42u32));
    println!("{:016x}", h(&"hello"));
    println!("{:016x}", h(&Suit::Spades));
    println!("{:016x}", h(&Shape::Rect { w: 1, h: 2 }));
    println!("{:016x}", h(&Version::default()));
    println!("{:016x}", h(&(1i8, 'x', true)));
    let bh: BuildHasherDefault<DefaultHasher> = BuildHasherDefault::default();
    println!("{:016x}", bh.hash_one([7u16, 8, 9]));
    let st = Settings::default();
    println!("{:?}", st);
    println!("{:?} {:?}", Mode::default(), Settings { level: 3, ..Default::default() }.level);
    let (a, b): (u8, String) = Default::default();
    println!("{:?} {:?} {:?}", a, b, <[u16; 3]>::default());
    println!("{:?}", <Option<Mode>>::default());
    println!("{:?}", Mode::Slow(1) != Mode::Fast);
}
