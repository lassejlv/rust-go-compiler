use std::slice;

fn split_at_mut_manual(s: &mut [i32], mid: usize) -> (&mut [i32], &mut [i32]) {
    let len = s.len();
    let p = s.as_mut_ptr();
    assert!(mid <= len);
    unsafe {
        (
            slice::from_raw_parts_mut(p, mid),
            slice::from_raw_parts_mut(p.add(mid), len - mid),
        )
    }
}

fn as_bytes<T: Copy>(v: &[T]) -> &[u8] {
    unsafe { slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v)) }
}

fn main() {
    let mut v = vec![1, 2, 3, 4, 5, 6];
    let (a, b) = split_at_mut_manual(&mut v, 2);
    a[0] = 100;
    b[0] = 300;
    b.reverse();
    println!("{:?}", v);

    let words = [0x0102u16, 0x0304];
    println!("{:?}", as_bytes(&words));

    let s = String::from("héllo wörld");
    let bytes = s.as_bytes();
    let sub = unsafe { slice::from_raw_parts(bytes.as_ptr().add(7), 6) };
    println!("{}", std::str::from_utf8(sub).unwrap());
    let st = unsafe { std::str::from_utf8_unchecked(&bytes[0..6]) };
    println!("{}", st);

    let empty: &[u64] = unsafe { slice::from_raw_parts(std::ptr::NonNull::dangling().as_ptr(), 0) };
    println!("empty len {}", empty.len());

    let mut buf = [0u8; 8];
    let p = buf.as_mut_ptr();
    unsafe {
        let s2 = slice::from_raw_parts_mut(p.add(2), 4);
        s2.copy_from_slice(&[9, 8, 7, 6]);
    }
    println!("{:?}", buf);
    let mut v2: Vec<u32> = Vec::with_capacity(4);
    unsafe {
        let p = v2.as_mut_ptr();
        for i in 0..4 {
            p.add(i).write((i as u32 + 1) * 11);
        }
        v2.set_len(4);
    }
    println!("{:?}", v2);
    let mut v3 = vec![1u8, 2, 3];
    let len = v3.len();
    let cap = v3.capacity();
    let p = v3.as_mut_ptr();
    std::mem::forget(v3);
    let v4 = unsafe { Vec::from_raw_parts(p, len, cap) };
    println!("{:?}", v4);
    let arr = [[1, 2], [3, 4], [5, 6]];
    let flat: &[i32] = unsafe { slice::from_raw_parts(arr.as_ptr() as *const i32, 6) };
    println!("{:?}", flat);
    let slice_ptr: *const [i32] = flat;
    println!("len via ptr {}", slice_ptr.len());
}
