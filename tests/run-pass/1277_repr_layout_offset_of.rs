// #[repr(C)], #[repr(u8)], #[repr(transparent)], packed/align layouts, offset_of!, transmute.
use std::mem::{align_of, offset_of, size_of, transmute};

#[repr(C)]
struct Header {
    tag: u8,
    len: u32,
    flags: u16,
    data: u64,
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

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
enum Color {
    Red = 1,
    Green = 2,
    Blue = 4,
    Custom = 200,
}

#[repr(i16)]
#[derive(Debug)]
enum Signed {
    Neg = -5,
    Zero = 0,
    Next,
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
struct Meters(f64);

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct Rgba {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

#[repr(u8)]
enum Tagged {
    A(u32) = 3,
    B { x: u16 } = 7,
    C = 9,
}

fn color_from(b: u8) -> Option<Color> {
    match b {
        1 | 2 | 4 | 200 => Some(unsafe { transmute::<u8, Color>(b) }),
        _ => None,
    }
}

fn main() {
    println!("Header size {} align {}", size_of::<Header>(), align_of::<Header>());
    println!("offsets {} {} {} {}", offset_of!(Header, tag), offset_of!(Header, len), offset_of!(Header, flags), offset_of!(Header, data));
    println!("Packed size {} align {} b@{} c@{}", size_of::<Packed>(), align_of::<Packed>(), offset_of!(Packed, b), offset_of!(Packed, c));
    let p = Packed { a: 1, b: 0x01020304, c: 0x0506 };
    let pb = p.b;
    let pc = p.c;
    println!("packed vals {} {:x} {:x}", p.a, pb, pc);
    println!("Aligned size {} align {}", size_of::<Aligned>(), align_of::<Aligned>());
    let al = [Aligned { x: 1 }, Aligned { x: 2 }];
    println!("aligned addr diff {}", (&al[1] as *const _ as usize) - (&al[0] as *const _ as usize));
    println!("al x {}", al[0].x + al[1].x);
    println!("Color size {} as u8 {} {}", size_of::<Color>(), Color::Blue as u8, Color::Custom as u8);
    println!("from {:?} {:?} {:?}", color_from(2), color_from(3), color_from(200));
    println!("Signed {} {} {} size {}", Signed::Neg as i16, Signed::Zero as i16, Signed::Next as i16, size_of::<Signed>());
    println!("{:?}", Signed::Next);
    println!("Meters size {} {:?}", size_of::<Meters>(), Meters(2.5));
    let m: f64 = unsafe { transmute(Meters(3.25)) };
    let back: Meters = unsafe { transmute(1.5f64) };
    println!("{} {:?}", m, back);
    let c = Rgba { r: 0x11, g: 0x22, b: 0x33, a: 0x44 };
    let as_u32: u32 = unsafe { transmute(c) };
    println!("rgba as u32 {:#x}", as_u32);
    let arr: [u8; 4] = unsafe { transmute(as_u32.to_be()) };
    println!("{:?}", arr);
    let f = 1.0f32;
    println!("f32 bits {:#x} {:#x}", unsafe { transmute::<f32, u32>(f) }, f.to_bits());
    let pair: [u16; 2] = unsafe { transmute(0x0001_0002u32) };
    println!("{:?}", pair);
    println!("Option<&u8> {} Option<Box<u8>> {} Option<NonZeroU32> {}", size_of::<Option<&u8>>(), size_of::<Option<Box<u8>>>(), size_of::<Option<std::num::NonZeroU32>>());
    println!("Tagged size {}", size_of::<Tagged>());
    for t in [Tagged::A(5), Tagged::B { x: 6 }, Tagged::C] {
        let tag = unsafe { *(&t as *const Tagged as *const u8) };
        let v = match t {
            Tagged::A(v) => v,
            Tagged::B { x } => x as u32,
            Tagged::C => 0,
        };
        println!("tag {} payload {}", tag, v);
    }
    println!("tuple {} {}", size_of::<(u8, u32, u8)>(), align_of::<(u8, u64)>());
    println!("unit {} empty arr {} fnptr {}", size_of::<()>(), size_of::<[u64; 0]>(), size_of::<fn(i32) -> i32>());
    println!("&[u8] {} &str {} &dyn Fn {}", size_of::<&[u8]>(), size_of::<&str>(), size_of::<&dyn Fn()>());
}
