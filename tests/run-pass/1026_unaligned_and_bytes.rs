use std::ptr;

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct Msg {
    tag: u8,
    id: u32,
    val: u64,
    tail: u16,
}

fn parse(buf: &[u8]) -> Msg {
    assert!(buf.len() >= std::mem::size_of::<Msg>());
    unsafe { ptr::read_unaligned(buf.as_ptr() as *const Msg) }
}

fn main() {
    let mut buf = vec![0u8; 32];
    buf[0] = 0xAB;
    buf[1..5].copy_from_slice(&0x11223344u32.to_le_bytes());
    buf[5..13].copy_from_slice(&0x0102030405060708u64.to_le_bytes());
    buf[13..15].copy_from_slice(&0xBEEFu16.to_le_bytes());
    let m = parse(&buf);
    let (tag, id, val, tail) = (m.tag, m.id, m.val, m.tail);
    println!("{:#x} {:#x} {:#x} {:#x} size={}", tag, id, val, tail, std::mem::size_of::<Msg>());
    let id_ptr = ptr::addr_of!(m.id);
    println!("addr_of read {:#x}", unsafe { id_ptr.read_unaligned() });
    let mut m2 = m;
    let vp = ptr::addr_of_mut!(m2.val);
    unsafe { vp.write_unaligned(42); }
    println!("val now {}", { m2.val });
    let raw = &raw const m2.tail;
    println!("raw const {:#x}", unsafe { raw.read_unaligned() });

    let p = buf.as_ptr();
    for off in 0..4 {
        let v = unsafe { (p.add(off) as *const u32).read_unaligned() };
        println!("u32 at {} = {:#010x}", off, v);
    }
    let mut out = [0u8; 10];
    unsafe {
        (out.as_mut_ptr().add(3) as *mut u32).write_unaligned(0xCAFEF00D);
        ptr::write_bytes(out.as_mut_ptr().add(7), 0xEE, 3);
    }
    println!("{:02x?}", out);
    let words = [1u32, 2, 3, 4];
    let bp = words.as_ptr() as *const u8;
    let off = unsafe { bp.add(1) }.align_offset(4);
    println!("align_offset {}", off);
    println!("is_aligned {}", words.as_ptr().is_aligned());
    let (pre, mid, post) = unsafe { buf[1..].align_to::<u32>() };
    println!("align_to parts {} {} {}", pre.len(), mid.len(), post.len());
    let x = 0x1234_5678u32;
    let bytes: [u8; 4] = unsafe { std::mem::transmute(x.to_be()) };
    println!("{:02x?}", bytes);
    let mut arr = [1u8, 2, 3, 4, 5, 6];
    unsafe {
        let p = arr.as_mut_ptr();
        ptr::swap_nonoverlapping(p, p.add(3), 3);
    }
    println!("{:?}", arr);
    let s = [10u16, 20, 30];
    let sp: *const [u16] = ptr::slice_from_raw_parts(s.as_ptr(), 2);
    println!("{:?}", unsafe { &*sp });
}
