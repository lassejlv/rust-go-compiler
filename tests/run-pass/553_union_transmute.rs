// unions with field access, repr(C) unions in structs, transmute between same-size types, to/from bits
use std::mem;

#[repr(C)]
union IntOrFloat {
    i: u32,
    f: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
union Bytes8 {
    u: u64,
    b: [u8; 8],
    h: [u16; 4],
    s: i64,
}

#[derive(Clone, Copy, PartialEq, Debug)]
#[repr(u8)]
enum Kind {
    Int = 1,
    Float = 2,
}

#[repr(C)]
struct Tagged {
    kind: Kind,
    val: IntOrFloat,
}

impl Tagged {
    fn show(&self) -> String {
        unsafe {
            match self.kind {
                Kind::Int => format!("int {}", self.val.i),
                Kind::Float => format!("float {}", self.val.f),
            }
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct Rgba {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
enum Level {
    Low = 10,
    Mid = 20,
    High = 30,
}

fn main() {
    let mut u = IntOrFloat { f: 1.0 };
    unsafe {
        println!("{:#010x}", u.i);
        u.i = 0x40490fdb;
        println!("{}", u.f);
        u.f = -0.0;
        println!("{:#x}", u.i);
    }
    let mut b = Bytes8 { u: 0x0102030405060708 };
    unsafe {
        println!("{:?}", b.b);
        println!("{:x?}", b.h);
        b.b[7] = 0xff;
        println!("{} {:#x}", b.s, b.u);
        b.h[0] = 0;
        println!("{:#018x}", b.u);
    }
    let copy = b;
    println!("{}", unsafe { copy.b[1] });
    let items = [Tagged { kind: Kind::Int, val: IntOrFloat { i: 42 } }, Tagged { kind: Kind::Float, val: IntOrFloat { f: 2.5 } }];
    for t in &items {
        println!("{}", t.show());
    }
    println!("{} {}", mem::size_of::<IntOrFloat>(), mem::size_of::<Tagged>());
    println!("{} {}", mem::size_of::<Bytes8>(), mem::align_of::<Bytes8>());
    let f = 3.5f64;
    let bits: u64 = unsafe { mem::transmute(f) };
    println!("{:#x} {}", bits, bits == f.to_bits());
    let back: f64 = unsafe { mem::transmute(bits) };
    println!("{}", back);
    let px: u32 = unsafe { mem::transmute(Rgba { r: 1, g: 2, b: 3, a: 4 }) };
    println!("{:#010x}", px);
    let c: Rgba = unsafe { mem::transmute(0xddccbbaau32) };
    println!("{:?}", c);
    let arr: [u8; 4] = unsafe { mem::transmute(0x01020304u32.to_be()) };
    println!("{:?}", arr);
    let words: [u16; 2] = unsafe { mem::transmute([1u8, 0, 2, 0]) };
    println!("{:?}", words);
    let lv: Level = unsafe { mem::transmute(20u8) };
    println!("{:?} {}", lv, Level::High as u8 + Level::Low as u8);
    let k: u8 = unsafe { mem::transmute(Kind::Float) };
    println!("{}", k);
    let signed: i32 = unsafe { mem::transmute(u32::MAX) };
    println!("{}", signed);
    let tup: (u32, u32) = unsafe { mem::transmute(0x0000000700000003u64) };
    println!("{:?}", tup);
    let fp: fn(i32) -> i32 = |x| x * 3;
    let raw: *const () = fp as *const ();
    let fp2: fn(i32) -> i32 = unsafe { mem::transmute(raw) };
    println!("{}", fp2(14));
    let opt_ref: Option<&u8> = unsafe { mem::transmute(0usize) };
    println!("{:?}", opt_ref);
    println!("{}", mem::size_of::<Option<&u8>>() == mem::size_of::<usize>());
    let sl: &[u32] = &[1, 2, 3];
    let (ptr, len): (*const u32, usize) = unsafe { mem::transmute(sl) };
    println!("{} {}", len, unsafe { *ptr.add(2) });
    let fl = [1.0f32, -2.0];
    let ints: [u32; 2] = unsafe { mem::transmute(fl) };
    println!("{:x?}", ints);
    println!("{}", f32::from_bits(ints[1]));
    let mut un = Bytes8 { s: -1 };
    unsafe {
        un.h[3] = 0x7fff;
        println!("{}", un.s);
    }
}
