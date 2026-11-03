use std::hash::{Hash, Hasher, BuildHasher};
use std::collections::HashMap;

struct Fnv(u64);
impl Hasher for Fnv {
    fn write(&mut self, bytes: &[u8]) {
        for b in bytes { self.0 ^= *b as u64; self.0 = self.0.wrapping_mul(0x100000001b3); }
    }
    fn finish(&self) -> u64 { self.0 }
}
#[derive(Default, Clone)]
struct FnvBuild;
impl BuildHasher for FnvBuild {
    type Hasher = Fnv;
    fn build_hasher(&self) -> Fnv { Fnv(0xcbf29ce484222325) }
}

struct Trace(Vec<usize>);
impl Hasher for Trace {
    fn write(&mut self, bytes: &[u8]) { self.0.push(bytes.len()); }
    fn finish(&self) -> u64 { 0 }
}

#[derive(Hash, PartialEq, Eq, Debug, Clone)]
struct Rec<T> { id: u32, tag: T, flags: (bool, u8) }

#[derive(Hash, PartialEq, Eq, Debug, Clone, Copy)]
enum Shape<N> { Dot, Line(N), Rect { w: N, h: N } }

fn h<T: Hash + ?Sized>(t: &T) -> u64 { let mut s = FnvBuild.build_hasher(); t.hash(&mut s); s.finish() }
fn trace<T: Hash + ?Sized>(t: &T) -> Vec<usize> { let mut s = Trace(vec![]); t.hash(&mut s); s.0 }

fn main() {
    println!("{:x}", h(&0u8));
    println!("{:x}", h(&1234567u32));
    println!("{:x}", h(&-5i64));
    println!("{:x}", h("hello"));
    println!("{:x}", h(&String::from("hello")));
    println!("{:x}", h(&(1u16, 'x')));
    println!("{:x}", h(&[1u8, 2, 3][..]));
    println!("{:x}", h(&vec![1u8, 2, 3]));
    println!("{:x}", h(&Rec { id: 7, tag: "t", flags: (true, 3) }));
    println!("{:x} {:x} {:x}", h(&Shape::<u8>::Dot), h(&Shape::Line(4u8)), h(&Shape::Rect { w: 1u16, h: 2 }));
    println!("{:x}", h(&Some(9i32)));
    println!("{:?}", trace(&5u32));
    println!("{:?}", trace("abc"));
    println!("{:?}", trace(&vec!["a", "bc"]));
    println!("{:?}", trace(&Rec { id: 1, tag: 2u64, flags: (false, 0) }));
    println!("{:?}", trace(&Shape::Rect { w: 1u8, h: 2 }));
    println!("{:?}", trace(&(Some(1u8), None::<u16>)));
    println!("{:?}", trace(&()));
    println!("{:?}", trace(&'c'));
    let mut m: HashMap<Rec<&str>, u32, FnvBuild> = HashMap::with_hasher(FnvBuild);
    for i in 0..5u32 { m.insert(Rec { id: i % 3, tag: "k", flags: (i % 2 == 0, 0) }, i); }
    let mut items: Vec<_> = m.iter().map(|(k, v)| (k.id, k.flags.0, *v)).collect();
    items.sort();
    println!("{:?}", items);
}
