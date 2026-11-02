// MaybeUninit arrays initialized element-wise, uninit_array-style patterns, assume_init_read,
// unions (repr(C)) for type punning f32<->u32 and tagged-union enums done by hand, union with
// ManuallyDrop fields, and size/align checks.
use std::mem::{self, ManuallyDrop, MaybeUninit};

#[repr(C)]
union FloatBits {
    f: f32,
    u: u32,
    bytes: [u8; 4],
}

#[repr(C)]
union Wide {
    d: f64,
    q: u64,
    halves: [u32; 2],
}

#[derive(Clone, Copy)]
#[repr(u8)]
enum Tag {
    Int = 1,
    Float = 2,
    Str = 3,
}

union Payload {
    i: i64,
    f: f64,
    s: ManuallyDrop<String>,
}

struct Tagged {
    tag: Tag,
    p: Payload,
}

impl Tagged {
    fn int(i: i64) -> Self {
        Tagged { tag: Tag::Int, p: Payload { i } }
    }
    fn float(f: f64) -> Self {
        Tagged { tag: Tag::Float, p: Payload { f } }
    }
    fn string(s: &str) -> Self {
        Tagged { tag: Tag::Str, p: Payload { s: ManuallyDrop::new(s.to_string()) } }
    }
    fn describe(&self) -> String {
        unsafe {
            match self.tag {
                Tag::Int => format!("int {}", self.p.i),
                Tag::Float => format!("float {}", self.p.f),
                Tag::Str => format!("str {:?}", &*self.p.s),
            }
        }
    }
}
impl Drop for Tagged {
    fn drop(&mut self) {
        if let Tag::Str = self.tag {
            let s = unsafe { ManuallyDrop::take(&mut self.p.s) };
            println!("  dropping string payload {:?}", s);
        }
    }
}

fn build_array<const N: usize>(f: impl Fn(usize) -> String) -> [String; N] {
    let mut arr: [MaybeUninit<String>; N] = [const { MaybeUninit::uninit() }; N];
    for (i, slot) in arr.iter_mut().enumerate() {
        slot.write(f(i));
    }
    // all initialized
    unsafe { mem::transmute_copy::<[MaybeUninit<String>; N], [String; N]>(&ManuallyDrop::new(arr)) }
}

fn main() {
    let mut fb = FloatBits { f: 1.0 };
    unsafe {
        println!("{:#010x} {:?}", fb.u, fb.bytes);
        fb.u ^= 0x8000_0000;
        println!("{}", fb.f);
        fb.bytes[3] = 0x40;
        fb.bytes[2] = 0x49;
        println!("{} {:#x}", fb.f, fb.u);
    }
    let w = Wide { d: -2.5 };
    unsafe {
        println!("{:#018x} {:x?} {}", w.q, w.halves, w.halves[1] >> 31);
    }
    println!("sizes {} {} {} {}", mem::size_of::<FloatBits>(), mem::size_of::<Wide>(), mem::size_of::<Payload>(), mem::align_of::<Wide>());

    let items = vec![Tagged::int(-42), Tagged::float(0.125), Tagged::string("hello union")];
    for t in &items {
        println!("{}", t.describe());
    }
    drop(items);
    println!("dropped items");

    let mut buf: [MaybeUninit<u32>; 5] = [MaybeUninit::uninit(); 5];
    for (i, b) in buf.iter_mut().enumerate() {
        b.write((i as u32 + 1) * 11);
    }
    let vals: Vec<u32> = buf.iter().map(|b| unsafe { b.assume_init_read() }).collect();
    println!("{:?}", vals);

    let strs: [String; 4] = build_array(|i| format!("s{}", i * i));
    println!("{:?}", strs);

    let mut x = MaybeUninit::<(u8, u16)>::uninit();
    let p = x.as_mut_ptr();
    unsafe {
        std::ptr::addr_of_mut!((*p).0).write(7);
        std::ptr::addr_of_mut!((*p).1).write(700);
    }
    let pair = unsafe { x.assume_init() };
    println!("{:?}", pair);

    let zeroed: (u64, bool, Option<&u8>) = unsafe { mem::zeroed() };
    println!("{:?}", zeroed);
    let z: MaybeUninit<[i16; 3]> = MaybeUninit::zeroed();
    println!("{:?}", unsafe { z.assume_init() });

    let mut slot = MaybeUninit::<Vec<i32>>::uninit();
    let vref = slot.write(vec![1, 2]);
    vref.push(3);
    let v = unsafe { slot.assume_init() };
    println!("{:?}", v);
    println!("{}", mem::size_of::<MaybeUninit<u64>>() == mem::size_of::<u64>());
    println!("{} {}", mem::size_of::<Option<Box<u8>>>() == mem::size_of::<usize>(), mem::size_of::<Option<&u32>>());
    println!("{} {}", mem::align_of::<u64>(), mem::size_of::<Tagged>() >= 16);
}
