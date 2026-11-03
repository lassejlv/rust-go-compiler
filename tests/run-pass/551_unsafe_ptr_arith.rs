// raw pointer arithmetic: add/sub/offset/offset_from, slices from_raw_parts(_mut), ptr::copy/swap/write/read
use std::ptr;

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
struct Pixel {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

unsafe fn sum_raw(p: *const i32, n: usize) -> i32 {
    let mut s = 0;
    for i in 0..n {
        s += *p.add(i);
    }
    s
}

fn reverse_in_place<T>(v: &mut [T]) {
    let n = v.len();
    let p = v.as_mut_ptr();
    unsafe {
        for i in 0..n / 2 {
            ptr::swap(p.add(i), p.add(n - 1 - i));
        }
    }
}

fn main() {
    let arr = [10, 20, 30, 40, 50, 60];
    let p = arr.as_ptr();
    unsafe {
        println!("{} {} {}", *p, *p.add(2), *p.offset(5));
        let end = p.add(arr.len());
        let mid = end.sub(3);
        println!("{} {}", *mid, end.offset_from(p));
        println!("{}", mid.offset_from(p.add(1)));
        println!("{}", sum_raw(p.add(1), 4));
        let s = std::slice::from_raw_parts(p.add(2), 3);
        println!("{:?}", s);
    }
    let mut v = vec![1u32, 2, 3, 4, 5, 6, 7];
    unsafe {
        let mp = v.as_mut_ptr();
        *mp.add(3) *= 100;
        mp.add(6).write(77);
        let r = mp.add(1).read();
        println!("read {}", r);
        let sm = std::slice::from_raw_parts_mut(mp.add(4), 2);
        sm[0] = 0;
        sm[1] += 1;
        ptr::copy(mp, mp.add(2), 3);
        println!("{:?}", v);
        let mp = v.as_mut_ptr();
        ptr::copy_nonoverlapping(mp.add(5), mp, 2);
        println!("{:?}", v);
        let old = ptr::replace(mp.add(1), 999);
        println!("replaced {} -> {:?}", old, v);
    }
    reverse_in_place(&mut v);
    println!("{:?}", v);
    let mut words = vec![String::from("a"), String::from("bb"), String::from("ccc")];
    reverse_in_place(&mut words);
    println!("{:?}", words);
    let mut px = [Pixel { r: 1, g: 2, b: 3, a: 4 }, Pixel { r: 5, g: 6, b: 7, a: 8 }];
    unsafe {
        let bytes = std::slice::from_raw_parts_mut(px.as_mut_ptr() as *mut u8, 8);
        println!("{:?}", bytes);
        bytes[4] = 255;
        let pp = px.as_ptr();
        println!("{:?}", (*pp.add(1)).r);
        let gp = ptr::addr_of!((*pp).g);
        println!("{}", *gp);
        let field_off = (ptr::addr_of!(px[0].b) as usize) - (pp as usize);
        println!("offset of b = {}", field_off);
    }
    println!("{:?}", px[1]);
    let mut a = 1;
    let mut b = 2;
    unsafe {
        ptr::swap(&mut a, &mut b);
    }
    println!("{} {}", a, b);
    let x = 0x11223344u32;
    let xp = &x as *const u32 as *const u8;
    let le: Vec<u8> = unsafe { (0..4).map(|i| *xp.add(i)).collect() };
    println!("{:x?}", le);
    let mut buf = [0u16; 5];
    let bp = buf.as_mut_ptr();
    unsafe {
        for i in 0..5 {
            bp.add(i).write((i * i) as u16);
        }
        ptr::write_bytes(bp.add(3), 0xff, 1);
    }
    println!("{:?}", buf);
    let null: *const i32 = ptr::null();
    println!("{} {}", null.is_null(), p.is_null());
    let nz = unsafe { p.as_ref() };
    println!("{:?}", nz);
    let nn: *const i32 = ptr::null();
    println!("{:?}", unsafe { nn.as_ref() });
    let s = "hello";
    let sp = s.as_ptr();
    let sub = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(sp.add(1), 3)) };
    println!("{}", sub);
    let v2: Vec<i64> = vec![5, 6, 7];
    let first = &v2[0] as *const i64;
    let third = &v2[2] as *const i64;
    println!("{}", unsafe { third.offset_from(first) });
    println!("{}", (third as usize - first as usize) / std::mem::size_of::<i64>());
    let cp = p.cast::<u8>();
    println!("{}", unsafe { *cp.add(4) });
    let wp = p.wrapping_add(100).wrapping_sub(99);
    println!("{}", unsafe { *wp });
    println!("{}", ptr::eq(&arr[1], unsafe { &*p.add(1) }));
}
