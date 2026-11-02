use std::ffi::c_void;
use std::mem::{align_of, size_of};

extern "C" {
    fn memset(dst: *mut c_void, c: i32, n: usize) -> *mut c_void;
    fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> i32;
    fn memmove(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct Header {
    tag: u8,
    len: u32,
    flags: u16,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

#[repr(C)]
struct Packet {
    header: Header,
    payload: [u8; 8],
}

#[repr(C, packed)]
struct Packed {
    a: u8,
    b: u32,
}

#[repr(C)]
struct Empty;

fn main() {
    println!("u8 {} u16 {} u32 {} u64 {} u128 {}", size_of::<u8>(), size_of::<u16>(), size_of::<u32>(), size_of::<u64>(), size_of::<u128>());
    println!("i8 {} f32 {} f64 {} char {} bool {}", size_of::<i8>(), size_of::<f32>(), size_of::<f64>(), size_of::<char>(), size_of::<bool>());
    println!("align u16 {} u32 {} f64 {}", align_of::<u16>(), align_of::<u32>(), align_of::<f64>());
    println!("Header size {} align {}", size_of::<Header>(), align_of::<Header>());
    println!("Vec3 size {} align {}", size_of::<Vec3>(), align_of::<Vec3>());
    println!("Packet size {}", size_of::<Packet>());
    println!("Packed size {} align {}", size_of::<Packed>(), align_of::<Packed>());
    println!("[u16; 5] size {}", size_of::<[u16; 5]>());
    println!("() size {} Empty {}", size_of::<()>(), size_of::<Empty>());
    println!("Option<Box<i32>> == Box<i32>: {}", size_of::<Option<Box<i32>>>() == size_of::<Box<i32>>());
    println!("Option<&u8> == &u8: {}", size_of::<Option<&u8>>() == size_of::<&u8>());
    println!("&[u8] is two words: {}", size_of::<&[u8]>() == 2 * size_of::<usize>());
    println!("Option<NonZeroU32> {}", size_of::<Option<std::num::NonZeroU32>>());

    let mut h = Header { tag: 7, len: 1234, flags: 0xFFFF };
    unsafe {
        memset(&mut h as *mut Header as *mut c_void, 0, size_of::<Header>());
    }
    println!("{:?}", h);

    let src = Vec3 { x: 1.5, y: -2.0, z: 3.25 };
    let mut dst = Vec3::default();
    unsafe {
        memcpy(&mut dst as *mut Vec3 as *mut c_void, &src as *const Vec3 as *const c_void, size_of::<Vec3>());
    }
    println!("{:?}", dst);
    let same = unsafe { memcmp(&src as *const _ as *const c_void, &dst as *const _ as *const c_void, size_of::<Vec3>()) };
    println!("memcmp equal: {}", same == 0);

    let mut buf = [0u8; 16];
    unsafe {
        memset(buf.as_mut_ptr() as *mut c_void, 0xAB, 4);
        memset(buf.as_mut_ptr().add(8) as *mut c_void, 1, 3);
    }
    println!("{:?}", buf);
    let mut text = *b"abcdefgh";
    unsafe {
        memmove(text.as_mut_ptr().add(2) as *mut c_void, text.as_ptr() as *const c_void, 5);
    }
    println!("{}", String::from_utf8_lossy(&text));

    let mut pkt = Packet { header: Header { tag: 1, len: 8, flags: 2 }, payload: [0; 8] };
    let data = b"PAYLOAD!";
    unsafe {
        memcpy(pkt.payload.as_mut_ptr() as *mut c_void, data.as_ptr() as *const c_void, 8);
    }
    println!("{:?} {}", pkt.header, std::str::from_utf8(&pkt.payload).unwrap());

    let p = Packed { a: 1, b: 0xDEADBEEF };
    let b = p.b;
    println!("packed {} {:#x}", p.a, b);
    let arr = [Vec3 { x: 1.0, y: 2.0, z: 3.0 }, Vec3 { x: 4.0, y: 5.0, z: 6.0 }];
    let floats: &[f32] = unsafe { std::slice::from_raw_parts(arr.as_ptr() as *const f32, 6) };
    println!("{:?}", floats);
    println!("offset of len: {}", std::mem::offset_of!(Header, len));
    println!("offset of flags: {}", std::mem::offset_of!(Header, flags));
}
