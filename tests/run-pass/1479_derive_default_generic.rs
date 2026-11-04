// derive(Default/Clone/Copy) with generics, bounds, PhantomData, fn pointers, #[default] variants
use std::marker::PhantomData;
use std::collections::BTreeMap;

#[derive(Debug, Default, Clone)]
struct Config<T, U = String> where T: Clone {
    items: Vec<T>,
    label: U,
    map: BTreeMap<String, T>,
    count: Option<usize>,
    _p: PhantomData<fn() -> T>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
enum Level { Low, #[default] Mid, High }

#[derive(Debug, Default)]
enum Wrapper<T> { #[default] Empty, Full(T) }

#[derive(Clone, Copy)]
struct Callbacks { on: fn(i32) -> i32, off: fn() -> &'static str }

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct Pt<T: Copy>(T, T);

#[derive(Debug, Default, Clone, PartialEq)]
struct Nested { lv: Level, pts: [Pt<u8>; 2], t: (bool, char, f32), s: &'static str }

fn main() {
    let c: Config<i32> = Default::default();
    println!("{:?}", c);
    let mut c2 = c.clone();
    c2.items.push(4);
    c2.label.push_str("lbl");
    println!("{:?} {:?}", c2.items, c2.label);
    println!("{:?} {:?}", Level::default(), Wrapper::<u8>::default());
    println!("{:?}", Wrapper::Full(3));
    let cb = Callbacks { on: |x| x + 1, off: || "off" };
    let cb2 = cb;
    println!("{} {}", (cb2.on)(1), (cb.off)());
    let p = Pt(1u8, 2);
    let q = p;
    println!("{:?} {:?} {}", p, q, p < Pt(1, 3));
    println!("{:?}", Nested::default());
    println!("{:#?}", Nested { lv: Level::High, ..Default::default() });
}
