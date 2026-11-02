// transmute of plain data (arrays <-> integers, repr(C) structs <-> byte arrays, fn pointers),
// repr(C)/repr(u8)/repr(transparent) layouts, field offsets via offset_of!, packed structs with
// read_unaligned, and serializing a repr(C) header to bytes and back.
use std::mem::{self, offset_of};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
struct Pixel {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct Record {
    kind: u8,
    // 3 bytes padding
    value: u32,
    scale: f32,
    count: u16,
    // 2 bytes padding
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct Packed {
    kind: u8,
    value: u32,
    count: u16,
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
struct Meters(f64);

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
enum Op {
    Nop = 0,
    Push = 1,
    Pop = 2,
    Jump = 0x10,
}

impl Op {
    fn from_u8(b: u8) -> Option<Op> {
        match b {
            0 | 1 | 2 | 0x10 => Some(unsafe { mem::transmute::<u8, Op>(b) }),
            _ => None,
        }
    }
}

fn add_one(x: i32) -> i32 {
    x + 1
}

fn main() {
    let px = Pixel { r: 0x11, g: 0x22, b: 0x33, a: 0x44 };
    let as_u32: u32 = unsafe { mem::transmute(px) };
    println!("{:#010x}", u32::from_le(as_u32));
    let back: Pixel = unsafe { mem::transmute(0xffeeddccu32.to_le()) };
    println!("{:?}", back);
    let arr: [u8; 4] = unsafe { mem::transmute(px) };
    println!("{:?}", arr);

    let words: [u16; 4] = [1, 2, 3, 4];
    let big: u64 = unsafe { mem::transmute(words) };
    println!("{:#018x}", u64::from_le(big));
    let f: f32 = unsafe { mem::transmute(0x40490fdbu32) };
    println!("{}", f);
    let i: i32 = unsafe { mem::transmute(-1.0f32) };
    println!("{:#x}", i);
    let pair: [i32; 2] = unsafe { mem::transmute(0x0000000500000003u64.to_le()) };
    println!("{:?}", pair);

    println!("Record size={} align={}", mem::size_of::<Record>(), mem::align_of::<Record>());
    println!("offsets kind={} value={} scale={} count={}", offset_of!(Record, kind), offset_of!(Record, value), offset_of!(Record, scale), offset_of!(Record, count));
    println!("Packed size={} align={} value@{} count@{}", mem::size_of::<Packed>(), mem::align_of::<Packed>(), offset_of!(Packed, value), offset_of!(Packed, count));
    println!("Meters size={} Op size={}", mem::size_of::<Meters>(), mem::size_of::<Op>());

    let rec = Record { kind: 7, value: 0x01020304, scale: 1.5, count: 9 };
    let bytes: [u8; 16] = unsafe { mem::transmute(rec) };
    // padding bytes are unspecified; print only the defined fields' bytes
    println!("kind {:?} value {:?} scale {:?} count {:?}", &bytes[0..1], &bytes[4..8], &bytes[8..12], &bytes[12..14]);

    let p = Packed { kind: 1, value: 0xAABBCCDD, count: 0x1234 };
    let pv = p.value;
    let pc = unsafe { std::ptr::addr_of!(p.count).read_unaligned() };
    println!("{:#x} {:#x}", pv, pc);
    let pbytes: [u8; 7] = unsafe { mem::transmute(p) };
    println!("{:02x?}", pbytes);
    let p2: Packed = unsafe { mem::transmute([2u8, 1, 0, 0, 0, 5, 0]) };
    let (k, v, c) = (p2.kind, p2.value, p2.count);
    println!("{} {} {}", k, v, c);

    let m = Meters(12.5);
    let raw: f64 = unsafe { mem::transmute(m) };
    let ms: Vec<Meters> = vec![Meters(1.0), Meters(2.0)];
    let fs: &[f64] = unsafe { std::slice::from_raw_parts(ms.as_ptr() as *const f64, ms.len()) };
    println!("{} {:?}", raw, fs);

    for b in [0u8, 1, 2, 3, 16, 255] {
        println!("{} -> {:?} {:?}", b, Op::from_u8(b), Op::from_u8(b).map(|o| o as u8));
    }

    let fp: fn(i32) -> i32 = add_one;
    let addr = fp as usize;
    let fp2: fn(i32) -> i32 = unsafe { mem::transmute(addr) };
    println!("{} {}", fp2(41), addr != 0);

    let pixels = vec![Pixel { r: 1, g: 2, b: 3, a: 4 }, Pixel { r: 5, g: 6, b: 7, a: 8 }];
    let flat: &[u8] = unsafe { std::slice::from_raw_parts(pixels.as_ptr() as *const u8, pixels.len() * 4) };
    println!("{:?}", flat);
    let gray: Vec<u8> = flat.chunks(4).map(|c| ((c[0] as u16 + c[1] as u16 + c[2] as u16) / 3) as u8).collect();
    println!("{:?}", gray);
    let u: [u32; 2] = unsafe { mem::transmute(pixels.as_slice().as_ptr().cast::<[Pixel; 2]>().read()) };
    println!("{:08x?}", u.map(u32::from_le));
}
