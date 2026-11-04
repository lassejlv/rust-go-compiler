// derives on generic structs/enums with where clauses, #[default], packed
use std::collections::hash_map::DefaultHasher;
use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
struct Pair<A, B> { a: A, b: B }

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Tree<T> where T: Ord {
    Leaf,
    Node(Box<Tree<T>>, T, Box<Tree<T>>),
}

#[derive(Debug, Default, Clone, PartialEq)]
enum Mode {
    Fast,
    #[default]
    Normal,
    Slow { factor: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[repr(C)]
struct ReprC { x: u8, y: u32 }

#[derive(Clone, Copy)]
#[repr(packed)]
struct Packed { a: u8, b: u32 }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
enum Small { A = 1, B = 5, C }

#[derive(Debug, Default)]
struct Wrapper<T: Default + std::fmt::Debug>(T, Vec<T>);

#[derive(Debug, PartialEq, PartialOrd)]
struct Floaty(f64, f32);

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
struct Unit;

fn h<T: Hash>(t: &T) -> u64 { let mut s = DefaultHasher::new(); t.hash(&mut s); s.finish() }

fn main() {
    let p = Pair { a: 1, b: "x" };
    let q = Pair { a: 1, b: "y" };
    println!("{:?} {:?} {} {:?}", p, Pair::<u8, bool>::default(), p < q, p.cmp(&q));
    println!("{}", h(&p) == h(&Pair { a: 1, b: "x" }));
    let t = Tree::Node(Box::new(Tree::Leaf), 5, Box::new(Tree::Node(Box::new(Tree::Leaf), 7, Box::new(Tree::Leaf))));
    println!("{:?}", t);
    println!("{:#?}", t);
    println!("{}", Tree::Leaf < t);
    println!("{:?} {:?}", Mode::default(), Mode::Slow { factor: 3 });
    println!("{:#?}", Mode::Slow { factor: 3 });
    println!("{:?}", ReprC::default());
    let pk = Packed { a: 1, b: 0x01020304 };
    let pk2 = pk;
    let b = pk2.b;
    println!("{} {}", pk2.a, b);
    let mut set = BTreeSet::new();
    set.insert(Small::C); set.insert(Small::A); set.insert(Small::B);
    println!("{:?} {}", set, Small::C as u8);
    println!("{:?}", Wrapper::<String>::default());
    println!("{:#?}", Wrapper(3, vec![1, 2]));
    println!("{:?}", Floaty(1.0, 2.5).partial_cmp(&Floaty(1.0, f32::NAN)));
    println!("{:?} {:?}", Unit, Unit::default());
    println!("{:#?}", Pair { a: Unit, b: (1, "s") });
    println!("{:#?}", (1, Some(Unit), [Mode::Fast]));
}
