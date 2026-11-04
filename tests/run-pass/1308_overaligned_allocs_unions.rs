// Over-aligned types in Box/Vec/Rc/Arc/locals and raw alloc with big alignments; generic
// unions; packed fields read through addr_of!; union pattern matching.
use std::alloc::{alloc, alloc_zeroed, dealloc, realloc, Layout};
use std::mem::{align_of, size_of};
use std::ptr;
use std::rc::Rc;
use std::sync::Arc;

#[repr(C, align(64))]
#[derive(Clone, Copy, Debug, PartialEq)]
struct Line {
    v: u8,
}

#[repr(align(32))]
#[derive(Clone, Debug)]
enum Big {
    A(u64),
    B(String),
}

#[repr(C)]
union Either<T: Copy, U: Copy> {
    t: T,
    u: U,
}

#[repr(C, packed)]
struct Wire {
    kind: u8,
    len: u32,
    crc: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
union Reg {
    full: u32,
    halves: [u16; 2],
    bytes: [u8; 4],
}

fn aligned<T>(p: *const T, a: usize) -> bool {
    p as usize % a == 0
}

fn main() {
    println!("Line size {} align {}", size_of::<Line>(), align_of::<Line>());
    let b = Box::new(Line { v: 1 });
    println!("box aligned {}", aligned(&*b, 64));
    let v = vec![Line { v: 2 }; 5];
    println!("vec aligned {} stride {}", aligned(v.as_ptr(), 64), (&v[1] as *const Line as usize) - (&v[0] as *const Line as usize));
    let mut v2: Vec<Line> = Vec::new();
    for i in 0..20 {
        v2.push(Line { v: i });
        if !aligned(v2.as_ptr(), 64) {
            println!("misaligned after push {}", i);
        }
    }
    println!("v2 sum {}", v2.iter().map(|l| l.v as u32).sum::<u32>());
    let r = Rc::new(Line { v: 3 });
    let a = Arc::new(Line { v: 4 });
    println!("rc aligned {} arc aligned {}", aligned(&*r, 64), aligned(&*a, 64));
    let local = [Line { v: 5 }, Line { v: 6 }];
    println!("local aligned {}", aligned(&local[0], 64));
    let bx: Box<[Line]> = vec![Line { v: 7 }; 3].into_boxed_slice();
    println!("boxed slice aligned {}", aligned(bx.as_ptr(), 64));
    println!("Big size {} align {}", size_of::<Big>(), align_of::<Big>());
    let bigs = vec![Big::A(1), Big::B("x".into()), Big::A(3)];
    println!("bigs aligned {} {:?}", aligned(bigs.as_ptr(), 32), bigs);

    for &al in &[8usize, 128, 4096] {
        let layout = Layout::from_size_align(100, al).unwrap();
        unsafe {
            let p = alloc_zeroed(layout);
            let ok = aligned(p, al) && std::slice::from_raw_parts(p, 100).iter().all(|&b| b == 0);
            ptr::write_bytes(p, 0xCD, 100);
            let p2 = realloc(p, layout, 5000);
            let still = aligned(p2, al) && *p2.add(99) == 0xCD;
            dealloc(p2, Layout::from_size_align(5000, al).unwrap());
            println!("align {} ok {} realloc ok {}", al, ok, still);
        }
    }
    unsafe {
        let l = Layout::new::<Line>();
        let p = alloc(l) as *mut Line;
        p.write(Line { v: 9 });
        println!("raw Line aligned {} v {}", aligned(p, 64), (*p).v);
        dealloc(p as *mut u8, l);
    }

    let e: Either<f32, u32> = Either { t: -0.0 };
    println!("bits {:#x}", unsafe { e.u });
    let e2: Either<[u8; 2], u16> = Either { u: 0x0102 };
    println!("bytes {:?}", unsafe { e2.t });
    println!("Either<u8,u64> size {}", size_of::<Either<u8, u64>>());

    let w = Wire { kind: 1, len: 0xAABBCCDD, crc: 0x1234 };
    let len = unsafe { ptr::addr_of!(w.len).read_unaligned() };
    let crc = w.crc;
    println!("packed {} {:#x} {:#x} size {}", w.kind, len, crc, size_of::<Wire>());
    let mut w2 = w;
    unsafe { ptr::addr_of_mut!(w2.len).write_unaligned(7) };
    let l2 = w2.len;
    println!("written {}", l2);

    let regs = [Reg { full: 0 }, Reg { halves: [1, 0] }, Reg { bytes: [0, 0, 0, 0x80] }, Reg { full: 42 }];
    for r in regs {
        let d = unsafe {
            match r {
                Reg { full: 0 } => "zero".to_string(),
                Reg { halves: [1, _] } => "low half one".to_string(),
                Reg { bytes: [_, _, _, 0x80] } => "top bit".to_string(),
                Reg { full } => format!("other {}", full),
            }
        };
        println!("{}", d);
    }
}
