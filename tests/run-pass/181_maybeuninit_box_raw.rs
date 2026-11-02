use std::mem::{ManuallyDrop, MaybeUninit};

struct Tracked(&'static str);
impl Drop for Tracked {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

#[derive(Debug)]
struct Big {
    id: u32,
    data: [u16; 4],
}

fn make_array() -> [String; 4] {
    let mut arr: [MaybeUninit<String>; 4] = [const { MaybeUninit::uninit() }; 4];
    for (i, slot) in arr.iter_mut().enumerate() {
        slot.write(format!("s{}", i * i));
    }
    unsafe { std::mem::transmute::<[MaybeUninit<String>; 4], [String; 4]>(arr) }
}

fn main() {
    let mut x = MaybeUninit::<u64>::uninit();
    x.write(12345);
    let x = unsafe { x.assume_init() };
    println!("{}", x);

    let arr = make_array();
    println!("{:?}", arr);

    let mut buf: [MaybeUninit<u8>; 8] = [MaybeUninit::uninit(); 8];
    let mut n = 0;
    for b in b"hey".iter() {
        buf[n].write(*b);
        n += 1;
    }
    let init: Vec<u8> = buf[..n].iter().map(|m| unsafe { m.assume_init() }).collect();
    println!("{}", String::from_utf8(init).unwrap());

    let zeroed: Big = unsafe { MaybeUninit::zeroed().assume_init() };
    println!("{:?}", zeroed);

    let b = Box::new(Big { id: 7, data: [1, 2, 3, 4] });
    let raw: *mut Big = Box::into_raw(b);
    unsafe {
        (*raw).id += 1;
        (*raw).data[3] = 40;
    }
    let back = unsafe { Box::from_raw(raw) };
    println!("{:?}", back);

    let t = Box::new(Tracked("boxed"));
    let leaked: &'static mut Tracked = Box::leak(t);
    println!("leaked {}", leaked.0);
    let reboxed = unsafe { Box::from_raw(leaked as *mut Tracked) };
    drop(reboxed);

    let md = ManuallyDrop::new(Tracked("manual"));
    println!("manual holds {}", md.0);
    let mut md2 = ManuallyDrop::new(Tracked("manual2"));
    unsafe { ManuallyDrop::drop(&mut md2) };
    let inner = ManuallyDrop::into_inner(ManuallyDrop::new(Tracked("unwrapped")));
    println!("got {}", inner.0);

    let forgotten = Tracked("forgotten");
    std::mem::forget(forgotten);

    let v = vec![1u32, 2, 3];
    let mut v = std::mem::ManuallyDrop::new(v);
    let (p, len, cap) = (v.as_mut_ptr(), v.len(), v.capacity());
    let rebuilt = unsafe { Vec::from_raw_parts(p, len, cap) };
    println!("{:?}", rebuilt);

    let s = String::from("raw string");
    let bs = s.into_boxed_str();
    let rawp: *mut str = Box::into_raw(bs);
    let bs2 = unsafe { Box::from_raw(rawp) };
    println!("{} {}", bs2, bs2.len());

    let slice_box: Box<[i32]> = vec![5, 6, 7].into_boxed_slice();
    let raw_slice: *mut [i32] = Box::into_raw(slice_box);
    unsafe {
        (*raw_slice)[0] = 50;
    }
    let restored = unsafe { Box::from_raw(raw_slice) };
    println!("{:?}", restored);
    println!("end");
}
