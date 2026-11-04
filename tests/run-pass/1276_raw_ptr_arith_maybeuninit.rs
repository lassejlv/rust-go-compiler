// Raw pointer arithmetic, MaybeUninit arrays, ptr::read/write/copy/swap/replace.
use std::mem::{self, MaybeUninit};
use std::ptr;

#[derive(Debug, Clone)]
struct Item {
    id: u32,
    name: String,
}

fn make_array<const N: usize>(f: impl Fn(usize) -> Item) -> [Item; N] {
    let mut arr: [MaybeUninit<Item>; N] = [const { MaybeUninit::uninit() }; N];
    for (i, slot) in arr.iter_mut().enumerate() {
        slot.write(f(i));
    }
    unsafe { mem::transmute_copy::<_, [Item; N]>(&mem::ManuallyDrop::new(arr)) }
}

fn main() {
    let mut v = [10i32, 20, 30, 40, 50, 60];
    let p = v.as_mut_ptr();
    unsafe {
        *p.add(2) += 1;
        let end = p.add(v.len());
        println!("offset_from {}", end.offset_from(p));
        let mid = p.offset(3);
        println!("mid {} back {}", *mid, *mid.offset(-1));
        ptr::swap(p, p.add(5));
        let old = ptr::replace(p.add(1), 99);
        println!("replaced {}", old);
        ptr::copy(p, p.add(1), 3); // overlapping move right
        println!("{:?}", v);
        let q = v.as_mut_ptr();
        ptr::copy_nonoverlapping(q.add(4), q, 2);
        println!("{:?}", v);
        ptr::write_bytes(q.add(3), 0, 1);
        println!("{:?}", v);
        let r = q.wrapping_add(10).wrapping_sub(9);
        println!("wrapping {}", *r);
    }

    // ptr::read/write of non-Copy values
    let mut items = vec![
        Item { id: 1, name: "one".into() },
        Item { id: 2, name: "two".into() },
    ];
    unsafe {
        let a = ptr::read(&items[0]);
        let b = ptr::read(&items[1]);
        ptr::write(&mut items[0], b);
        ptr::write(&mut items[1], a);
    }
    println!("{:?}", items);

    let arr: [Item; 4] = make_array(|i| Item { id: i as u32 * 3, name: format!("n{}", i) });
    println!("{:?}", arr.iter().map(|i| (i.id, i.name.as_str())).collect::<Vec<_>>());

    let mut buf: [MaybeUninit<u16>; 5] = [MaybeUninit::uninit(); 5];
    for (i, b) in buf.iter_mut().enumerate() {
        *b = MaybeUninit::new((i * i) as u16);
    }
    let init: [u16; 5] = unsafe { mem::transmute(buf) };
    println!("{:?}", init);

    let mut x = MaybeUninit::<[u64; 3]>::uninit();
    let xp = x.as_mut_ptr() as *mut u64;
    unsafe {
        for i in 0..3 {
            xp.add(i).write(i as u64 * 100);
        }
        let x = x.assume_init();
        println!("{:?}", x);
    }
    let z: u32 = unsafe { MaybeUninit::zeroed().assume_init() };
    println!("zeroed {}", z);

    // read_unaligned / write_unaligned on a byte buffer
    let mut bytes = [0u8; 11];
    unsafe {
        let p = bytes.as_mut_ptr().add(1) as *mut u32;
        p.write_unaligned(0xdeadbeef);
        let q = bytes.as_ptr().add(5) as *const u16;
        (bytes.as_mut_ptr().add(5) as *mut u16).write_unaligned(0x1234);
        println!("{:x} {:x}", (bytes.as_ptr().add(1) as *const u32).read_unaligned(), q.read_unaligned());
    }
    println!("{:?}", bytes);
    let s = [1u8, 2, 3];
    let sp: *const [u8] = &s[..];
    println!("len {} null {}", sp.len(), sp.is_null());
    let np: *const i32 = ptr::null();
    println!("null {} {:?}", np.is_null(), unsafe { np.as_ref() });
    println!("eq {}", ptr::eq(&s[0], &s[0]));
}
