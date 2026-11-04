// derives on packed / aligned / repr(C) structs (packed fields are copied, never referenced)
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
#[repr(C, packed)]
struct Header { tag: u8, len: u32, flags: u16 }

#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[repr(packed(2))]
struct P2 { a: u8, b: u64, c: f32 }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(C, align(16))]
struct Aligned { x: u32, y: [u8; 3] }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(packed)]
struct Tup(u8, u32, i16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(i8)]
enum Signed { Neg = -1, Zero = 0, Pos = 1 }

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
enum Tagged { A(u8), B { x: u16 }, C }

fn h<T: Hash>(t: &T) -> u64 { let mut s = DefaultHasher::new(); t.hash(&mut s); s.finish() }

fn main() {
    let hd = Header { tag: 7, len: 0x0102_0304, flags: 0xffff };
    println!("{:?} {}", hd, std::mem::size_of::<Header>());
    println!("{:#?}", hd);
    let hd2 = Header { len: 5, ..hd };
    println!("{} {:?} {}", hd == hd2, hd.cmp(&hd2), h(&hd) == h(&Header { tag: 7, len: 0x0102_0304, flags: 0xffff }));
    println!("{:?}", Header::default());
    let p = P2 { a: 1, b: u64::MAX, c: 1.5 };
    println!("{:?} {} {}", p, std::mem::size_of::<P2>(), std::mem::align_of::<P2>());
    println!("{}", p == P2 { a: 1, b: u64::MAX, c: 1.5 });
    let al = Aligned { x: 1, y: [1, 2, 3] };
    println!("{:?} {} {}", al, std::mem::size_of::<Aligned>(), std::mem::align_of::<Aligned>());
    println!("{:?} {}", Tup(1, 2, -3), Tup(1, 2, -3) == Tup(1, 2, 3));
    let mut s = vec![Signed::Pos, Signed::Neg, Signed::Zero];
    s.sort();
    println!("{:?} {:?}", s, s.iter().map(|x| *x as i8).collect::<Vec<_>>());
    println!("{:?} {:?} {}", Tagged::A(1), Tagged::B { x: 2 }, Tagged::C == Tagged::C);
}
