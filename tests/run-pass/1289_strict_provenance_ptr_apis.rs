// Pointer APIs: addr_of!, cast/cast_mut/cast_const, addr/with_addr/map_addr, align_offset,
// is_aligned, NonNull helpers, as_ptr_range, volatile, copy_to/copy_from, mem::discriminant/needs_drop.
use std::mem;
use std::ptr::{self, NonNull};

#[repr(C)]
struct S {
    a: u8,
    b: u32,
    inner: Inner,
}
#[repr(C)]
struct Inner {
    x: u16,
    y: u64,
}

#[derive(Debug)]
enum E {
    A(u32),
    B { s: String },
    C,
}

fn main() {
    let s = S { a: 1, b: 2, inner: Inner { x: 3, y: 4 } };
    let base = ptr::addr_of!(s) as usize;
    let pb = ptr::addr_of!(s.b) as usize;
    let py = ptr::addr_of!(s.inner.y) as usize;
    println!("b@{} y@{} offset_of nested {}", pb - base, py - base, mem::offset_of!(S, inner.y));
    println!("a={} x={}", s.a, s.inner.x);
    let mut m = S { a: 0, b: 0, inner: Inner { x: 0, y: 0 } };
    unsafe {
        ptr::addr_of_mut!(m.b).write(99);
        (&raw mut m.inner.x).write(7);
    }
    println!("m.b={} m.inner.x={} y={}", m.b, m.inner.x, m.inner.y);

    let arr = [1u32, 2, 3, 4];
    let p: *const u32 = arr.as_ptr();
    let bytes: *const u8 = p.cast::<u8>();
    unsafe { println!("first byte {}", *bytes) };
    let pm = p.cast_mut();
    let pc = pm.cast_const();
    println!("same {}", pc == p);
    let a0 = p.addr();
    let p2 = p.with_addr(a0 + 8);
    unsafe { println!("with_addr {}", *p2) };
    let p3 = p.map_addr(|a| a + 4);
    unsafe { println!("map_addr {}", *p3) };
    println!("align_offset {} {}", bytes.wrapping_add(1).align_offset(4), p.align_offset(4));
    println!("aligned {} {}", p.is_aligned(), bytes.wrapping_add(1).cast::<u32>().is_aligned());
    let range = arr.as_ptr_range();
    println!("range len {}", unsafe { range.end.offset_from(range.start) });
    println!("contains {}", range.contains(&ptr::addr_of!(arr[2])));
    let fr = ptr::from_ref(&arr[1]);
    unsafe { println!("from_ref {}", *fr) };

    let mut data = [10i32, 20, 30];
    let nn = NonNull::from(&mut data).cast::<i32>();
    unsafe {
        let second = nn.add(1);
        *second.as_ptr() += 5;
        println!("nonnull read {}", second.read());
        let sl = NonNull::slice_from_raw_parts(nn, 3);
        println!("slice len {} {:?}", sl.len(), sl.as_ref());
        let byte_nn: NonNull<u8> = nn.cast();
        println!("byte {}", *byte_nn.as_ptr());
    }

    let mut dst = [0u16; 4];
    let src = [9u16, 8, 7, 6];
    unsafe {
        src.as_ptr().copy_to(dst.as_mut_ptr(), 2);
        dst.as_mut_ptr().add(2).copy_from(src.as_ptr().add(2), 2);
        ptr::write_volatile(dst.as_mut_ptr(), 100);
        println!("{:?} volatile {}", dst, ptr::read_volatile(&dst[3]));
    }

    let e1 = E::A(1);
    let e2 = E::A(2);
    let e3 = E::B { s: "x".into() };
    println!("disc eq {} {}", mem::discriminant(&e1) == mem::discriminant(&e2), mem::discriminant(&e1) == mem::discriminant(&e3));
    println!("{:?} {:?}", e3, E::C);
    println!("needs_drop {} {} {} {}", mem::needs_drop::<u32>(), mem::needs_drop::<String>(), mem::needs_drop::<E>(), mem::needs_drop::<[u8; 4]>());
    let dynv: &dyn std::fmt::Debug = &[0u64; 3];
    println!("size_of_val dyn {} align {}", mem::size_of_val(dynv), mem::align_of_val(dynv));
    println!("sv slice {}", mem::size_of_val(&arr[1..]));
    let tr: &[u8; 4] = &[1, 0, 0, 0];
    let as_u32: u32 = u32::from_le_bytes(*tr);
    let viaptr = unsafe { (tr as *const [u8; 4] as *const u32).read_unaligned() };
    println!("{} {}", as_u32, viaptr == u32::from_ne_bytes(*tr));
    let fnp: fn(u32) -> u32 = |x| x + 1;
    let raw = fnp as *const ();
    let back: fn(u32) -> u32 = unsafe { mem::transmute(raw) };
    println!("fnptr roundtrip {}", back(41));
    let sw = ptr::without_provenance::<u8>(0x1000);
    println!("without_provenance addr {:#x}", sw.addr());
    let dangling = ptr::dangling::<u64>();
    println!("dangling {}", dangling.addr() == mem::align_of::<u64>());
}
