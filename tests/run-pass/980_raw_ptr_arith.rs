use std::ptr;

fn sum_raw(p: *const i32, n: usize) -> i32 {
    let mut s = 0;
    unsafe {
        for i in 0..n {
            s += *p.add(i);
        }
    }
    s
}

fn main() {
    let mut arr = [10i32, 20, 30, 40, 50];
    let p = arr.as_mut_ptr();
    unsafe {
        *p.add(2) += 5;
        let q = p.add(4);
        println!("offset_from = {}", q.offset_from(p));
        let back = q.sub(3);
        println!("back = {}", *back);
        let w = p.wrapping_add(1);
        *w = -1;
        println!("{:?}", arr);
        let r = p.offset(3);
        ptr::write(r, 99);
        println!("read = {}", ptr::read(r));
    }
    println!("sum = {}", sum_raw(arr.as_ptr(), arr.len()));
    let x = 7u64;
    let px: *const u64 = &x;
    let pb = px as *const u8;
    let bytes: Vec<u8> = (0..8).map(|i| unsafe { *pb.add(i) }).collect();
    println!("le bytes = {:?}", bytes);
    println!("null? {} {}", px.is_null(), ptr::null::<u8>().is_null());
    let mut a = 1;
    let mut b = 2;
    unsafe { ptr::swap(&mut a, &mut b); }
    println!("a={} b={}", a, b);
    let mut src = [1u16, 2, 3, 4];
    let mut dst = [0u16; 6];
    unsafe {
        ptr::copy_nonoverlapping(src.as_ptr(), dst.as_mut_ptr().add(1), 4);
        ptr::copy(src.as_ptr(), src.as_mut_ptr().add(1), 3);
    }
    println!("dst={:?} src={:?}", dst, src);
    let old = unsafe { ptr::replace(&mut dst[0], 77) };
    println!("old={} dst0={}", old, dst[0]);
    let mut v = vec![String::from("a"), String::from("b")];
    unsafe {
        let s = ptr::read(&v[0]);
        ptr::write(&mut v[0], String::from("z"));
        // s now owns the original "a"
        println!("s={} v={:?}", s, v);
    }
    let eq = ptr::eq(&arr[0], &arr[0]);
    let ne = ptr::eq(&arr[0], &arr[1]);
    println!("eq {} {}", eq, ne);
    let addr_diff = (&arr[3] as *const i32 as usize) - (&arr[0] as *const i32 as usize);
    println!("addr diff {}", addr_diff);
}
