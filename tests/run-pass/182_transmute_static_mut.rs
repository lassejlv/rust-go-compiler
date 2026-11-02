use std::mem;

static mut TABLE: [u32; 8] = [0; 8];
static mut CALLS: usize = 0;
static GREETING: &str = "hi";

fn record(i: usize, v: u32) {
    unsafe {
        TABLE[i % 8] = v;
        CALLS += 1;
    }
}

fn table_snapshot() -> [u32; 8] {
    unsafe { *std::ptr::addr_of!(TABLE) }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct Rgba {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

fn main() {
    let x: u32 = 0x12345678;
    let bytes: [u8; 4] = unsafe { mem::transmute(x) };
    println!("{:?} == {:?}: {}", bytes, x.to_ne_bytes(), bytes == x.to_ne_bytes());
    println!("le {:?}", x.to_le_bytes());
    println!("be {:?}", x.to_be_bytes());
    println!("{:#x} {:#x}", u32::from_le_bytes([0x78, 0x56, 0x34, 0x12]), u32::from_be_bytes([0x78, 0x56, 0x34, 0x12]));
    let back: u32 = unsafe { mem::transmute(bytes) };
    println!("{:#x}", back);
    let f: f32 = unsafe { mem::transmute(0x3f800000u32) };
    println!("{}", f);
    let bits: u64 = unsafe { mem::transmute(-0.5f64) };
    println!("{:#x} {}", bits, bits == (-0.5f64).to_bits());
    let i: i32 = unsafe { mem::transmute(u32::MAX) };
    println!("{}", i);
    let c = Rgba { r: 1, g: 2, b: 3, a: 255 };
    let as_u32: u32 = unsafe { mem::transmute(c) };
    println!("{:?} -> {:#010x}", c, u32::from_le(as_u32));
    let back: Rgba = unsafe { mem::transmute(as_u32) };
    println!("{:?}", back);
    let arr: [u16; 2] = unsafe { mem::transmute([1u8, 0, 2, 0]) };
    println!("{:?}", arr.map(u16::from_le));
    println!("{:?}", 258u16.to_le_bytes());
    println!("{}", i64::from_le_bytes([0xff; 8]));
    println!("{:?}", (-2i16).to_be_bytes());
    println!("{:?}", 1.5f64.to_le_bytes());
    println!("{}", f64::from_le_bytes(1.5f64.to_le_bytes()));

    for i in 0..12 {
        record(i, (i * i) as u32);
    }
    println!("{:?}", table_snapshot());
    println!("calls {}", unsafe { CALLS });
    unsafe {
        let p = std::ptr::addr_of_mut!(TABLE);
        (*p).reverse();
        (*p)[0] += 1000;
    }
    println!("{:?}", table_snapshot());
    println!("{}", GREETING);

    let fp: fn(usize, u32) = record;
    let addr: usize = fp as usize;
    let fp2: fn(usize, u32) = unsafe { mem::transmute(addr) };
    fp2(3, 333);
    println!("via transmuted fn ptr: {}", table_snapshot()[3]);
    println!("{}", mem::size_of_val(&table_snapshot()));
    println!("{}", mem::size_of_val("héllo"));
    println!("{}", mem::align_of_val(&1u64));
    let disc_a = mem::discriminant(&Some(1));
    let disc_b = mem::discriminant(&Some(99));
    let disc_c = mem::discriminant(&None::<i32>);
    println!("{} {}", disc_a == disc_b, disc_a == disc_c);
}
