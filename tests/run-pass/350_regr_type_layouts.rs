// Regression: rustc-compatible type layouts (small tags, niche filling,
// field reordering) and that values in them behave correctly.
use std::cmp::Ordering;
use std::mem::{size_of, transmute};
use std::rc::Rc;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
enum Mid { A, B(bool), C, D }

#[allow(dead_code)]
#[derive(Debug)]
enum Wide { X = 1000, Y = 2000 }

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
enum Neg { M = -5, Z = 0, P = 5 }

#[allow(dead_code)]
#[derive(Debug)]
#[repr(u8)]
enum Prim { A(u32) = 1, B { x: u8, y: u16 } = 7, C = 9 }

#[allow(dead_code)]
#[derive(Debug)]
#[repr(C)]
enum CEnum { A(u8), B(u64) }

#[allow(dead_code)]
#[derive(Debug)]
#[repr(i16)]
enum Small { A = -300, B = 300 }

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
struct Packed3 { a: u8, b: u32, c: u8, d: u16 }

#[allow(dead_code)]
#[repr(C)]
struct InOrder { a: u8, b: u32, c: u8 }

#[derive(Debug)]
enum Tree { Leaf(u8), Node(Box<Tree>, Box<Tree>) }

fn depth(t: &Tree) -> u32 {
    match t {
        Tree::Leaf(_) => 1,
        Tree::Node(l, r) => 1 + depth(l).max(depth(r)),
    }
}

fn main() {
    println!("sizes: Mid={} Option<Mid>={} Wide={} Neg={} Prim={} CEnum={} Small={}",
        size_of::<Mid>(), size_of::<Option<Mid>>(), size_of::<Wide>(), size_of::<Neg>(),
        size_of::<Prim>(), size_of::<CEnum>(), size_of::<Small>());
    println!("sizes: Packed3={} InOrder={} (u8,u32,u8)={} Option<char>={} Option<Option<char>>={}",
        size_of::<Packed3>(), size_of::<InOrder>(), size_of::<(u8, u32, u8)>(),
        size_of::<Option<char>>(), size_of::<Option<Option<char>>>());
    println!("sizes: Option<Vec<u8>>={} Option<String>={} Option<Rc<u8>>={} Option<Box<Tree>>={} Tree={}",
        size_of::<Option<Vec<u8>>>(), size_of::<Option<String>>(), size_of::<Option<Rc<u8>>>(),
        size_of::<Option<Box<Tree>>>(), size_of::<Tree>());

    // niche-filled values with the data variant in the middle
    let all = [Mid::A, Mid::B(false), Mid::B(true), Mid::C, Mid::D];
    for m in all {
        let o = Some(m);
        let tag = match m { Mid::A => 'a', Mid::B(true) => 'T', Mid::B(false) => 'F', Mid::C => 'c', Mid::D => 'd' };
        print!("{}{} ", tag, if o.is_some() { "+" } else { "-" });
    }
    let none: Option<Mid> = None;
    println!("| none={:?} eq={}", none, none == None);

    // nested options over bool / char / Ordering
    let vals: [Option<Option<bool>>; 4] = [None, Some(None), Some(Some(false)), Some(Some(true))];
    println!("{:?}", vals);
    let cs: [Option<Option<char>>; 3] = [None, Some(None), Some(Some('\u{10FFFF}'))];
    println!("{:?}", cs);
    let ords = [None, Some(Ordering::Less), Some(Ordering::Equal), Some(Ordering::Greater)];
    println!("{:?} {}", ords, size_of::<[Option<Ordering>; 4]>());
    unsafe {
        let b: u8 = transmute(None::<bool>);
        let o: i8 = transmute(Ordering::Less);
        let on: u8 = transmute(None::<Ordering>);
        let some_true: u8 = transmute(Some(true));
        println!("transmute: None::<bool>={} Less={} None::<Ordering>={} Some(true)={}", b, o, on, some_true);
    }

    // discriminants
    println!("{} {} | {} {} {}", Wide::X as i32, Wide::Y as i32, Neg::M as i32, Neg::Z as i32, Neg::P as i32);
    let negs = [Neg::P, Neg::M, Neg::Z];
    let s: i32 = negs.iter().map(|n| *n as i32).sum();
    println!("neg sum {} small {} {}", s, Small::A as i16, Small::B as i16);
    let prims = [Prim::A(70000), Prim::B { x: 3, y: 65535 }, Prim::C];
    for p in &prims {
        let d = unsafe { *(p as *const Prim as *const u8) };
        print!("{:?}/{} ", p, d);
    }
    println!();
    let ce = [CEnum::A(9), CEnum::B(1 << 40)];
    println!("{:?}", ce);

    // reordered structs still behave
    let mut p = Packed3 { a: 1, b: 0xdeadbeef, c: 3, d: 4 };
    p.b ^= 0xffff;
    let q = p;
    println!("{:?} {}", q, q == p);
    let t = (1u8, 2u32, 3u8);
    let (x, y, z) = t;
    println!("{} {} {}", x, y, z);

    // pointer niches
    let mut ov: Option<Vec<u8>> = None;
    for i in 0..3 {
        ov.get_or_insert_with(Vec::new).push(i);
    }
    println!("{:?} {:?}", ov, ov.as_ref().map(|v| v.len()));
    let os: Option<String> = Some(String::new());
    println!("{:?} {}", os, os.is_some());
    let rc = Rc::new(5u8);
    let orc = Some(Rc::clone(&rc));
    println!("{:?} count {}", orc, Rc::strong_count(&rc));
    drop(orc);
    println!("count {}", Rc::strong_count(&rc));
    let mut taken = Some(vec![1, 2]);
    let v = taken.take();
    println!("{:?} {:?}", taken, v);

    let t = Tree::Node(Box::new(Tree::Leaf(1)), Box::new(Tree::Node(Box::new(Tree::Leaf(2)), Box::new(Tree::Leaf(3)))));
    println!("depth {}", depth(&t));
    let results: Vec<Result<u8, Ordering>> = vec![Ok(1), Err(Ordering::Less), Ok(255)];
    println!("{:?} {}", results, size_of::<Result<u8, Ordering>>());
}
