use std::mem::{align_of, size_of};

#[repr(C)]
struct CStruct {
    a: u8,
    b: u32,
    c: u16,
}

#[repr(C, packed)]
struct Packed {
    a: u8,
    b: u32,
    c: u16,
}

#[repr(C, align(16))]
struct Aligned {
    x: u8,
}

#[repr(transparent)]
struct Meters(f64);

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
enum Color {
    Red = 1,
    Green = 5,
    Blue,
}

#[repr(i32)]
enum Big {
    Neg = -3,
    Pos = 1000,
}

#[repr(C)]
struct Header {
    magic: u32,
    len: u16,
    flags: u8,
    kind: u8,
}

fn main() {
    println!("CStruct {} {}", size_of::<CStruct>(), align_of::<CStruct>());
    println!("Packed {} {}", size_of::<Packed>(), align_of::<Packed>());
    println!("Aligned {} {}", size_of::<Aligned>(), align_of::<Aligned>());
    println!("Meters {} {}", size_of::<Meters>(), align_of::<Meters>());
    println!("Color {} Big {}", size_of::<Color>(), size_of::<Big>());
    println!("Option<Box<u8>> {} Option<&u32> {}", size_of::<Option<Box<u8>>>(), size_of::<Option<&u32>>());
    println!("Option<std::num::NonZeroU32> {}", size_of::<Option<std::num::NonZeroU32>>());
    println!("(u8,u64) align {}", align_of::<(u8, u64)>());
    println!("[u16;5] {}", size_of::<[u16; 5]>());
    println!("unit {} ", size_of::<()>());

    let c = CStruct { a: 1, b: 2, c: 3 };
    let base = &c as *const CStruct as usize;
    println!("offsets: a={} b={} c={}",
        &c.a as *const u8 as usize - base,
        &c.b as *const u32 as usize - base,
        &c.c as *const u16 as usize - base);
    println!("offset_of: {} {} {}", std::mem::offset_of!(CStruct, a), std::mem::offset_of!(CStruct, b), std::mem::offset_of!(CStruct, c));
    println!("packed offset_of: {} {}", std::mem::offset_of!(Packed, b), std::mem::offset_of!(Packed, c));

    let p = Packed { a: 9, b: 0x11223344, c: 7 };
    let b = p.b;
    let raw = &p as *const Packed as *const u8;
    let bytes: Vec<u8> = (0..7).map(|i| unsafe { *raw.add(i) }).collect();
    println!("packed b={:#x} bytes={:02x?} a={} c={}", b, bytes, { p.a }, { p.c });

    let h = Header { magic: 0xCAFEBABE, len: 0x1234, flags: 0xAA, kind: 0x55 };
    let hb: [u8; 8] = unsafe { std::mem::transmute(h) };
    println!("header bytes {:02x?}", hb);

    println!("colors {} {} {}", Color::Red as u8, Color::Green as u8, Color::Blue as u8);
    let raw_c: u8 = 6;
    let col: Color = unsafe { std::mem::transmute(raw_c) };
    println!("{:?} {}", col, col == Color::Blue);
    println!("big {} {}", Big::Neg as i32, Big::Pos as i32);

    let m = Meters(3.5);
    let f: f64 = unsafe { std::mem::transmute(m) };
    println!("meters as f64 {}", f);
}
