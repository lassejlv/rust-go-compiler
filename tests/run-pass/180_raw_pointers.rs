use std::ptr::{self, NonNull};

struct Node {
    val: i32,
    next: Option<NonNull<Node>>,
}

fn main() {
    let mut v = vec![10, 20, 30, 40, 50];
    let p = v.as_mut_ptr();
    unsafe {
        println!("{} {}", *p, *p.add(4));
        *p.add(1) += 1;
        let third = p.offset(2);
        *third = -*third;
        let end = p.add(v.len());
        let mut cur = p;
        let mut sum = 0;
        while cur != end {
            sum += *cur;
            cur = cur.add(1);
        }
        println!("sum via ptr walk {}", sum);
        println!("distance {}", end.offset_from(p));
    }
    println!("{:?}", v);

    let s: &[i32] = unsafe { std::slice::from_raw_parts(v.as_ptr().add(1), 3) };
    println!("{:?}", s);
    let m: &mut [i32] = unsafe { std::slice::from_raw_parts_mut(v.as_mut_ptr(), 2) };
    m[0] = 99;
    println!("{:?}", v);

    let mut a = 1;
    let mut b = 2;
    unsafe {
        ptr::swap(&mut a, &mut b);
    }
    println!("swapped {} {}", a, b);
    let x = unsafe { ptr::replace(&mut a, 77) };
    println!("replace returned {} now {}", x, a);
    let mut arr = [1u8, 2, 3, 4, 5, 6];
    unsafe {
        ptr::copy(arr.as_ptr(), arr.as_mut_ptr().add(2), 3);
    }
    println!("{:?}", arr);
    let src = [9u8; 2];
    unsafe {
        ptr::copy_nonoverlapping(src.as_ptr(), arr.as_mut_ptr(), 2);
    }
    println!("{:?}", arr);
    let val = unsafe { ptr::read(&arr[3]) };
    unsafe { ptr::write(&mut arr[5], 200) };
    println!("{} {:?}", val, arr);

    let null: *const i32 = ptr::null();
    println!("null? {} {:?}", null.is_null(), unsafe { null.as_ref() });
    let r = &v[0] as *const i32;
    println!("as_ref {:?}", unsafe { r.as_ref() });
    println!("ptr::eq same {}", ptr::eq(&v[0], &v[0]));
    println!("ptr::eq diff {}", ptr::eq(&v[0], &v[1]));

    // Manual linked list with NonNull + Box::into_raw / from_raw.
    let mut head: Option<NonNull<Node>> = None;
    for i in 1..=4 {
        let node = Box::new(Node { val: i * 100, next: head });
        head = Some(unsafe { NonNull::new_unchecked(Box::into_raw(node)) });
    }
    let mut cur = head;
    let mut vals = vec![];
    while let Some(nn) = cur {
        unsafe {
            vals.push((*nn.as_ptr()).val);
            cur = nn.as_ref().next;
        }
    }
    println!("{:?}", vals);
    if let Some(mut h) = head {
        unsafe { h.as_mut().val += 1 };
    }
    let mut cur = head;
    let mut freed = 0;
    while let Some(nn) = cur {
        let b = unsafe { Box::from_raw(nn.as_ptr()) };
        if freed == 0 {
            println!("head val {}", b.val);
        }
        cur = b.next;
        freed += 1;
    }
    println!("freed {}", freed);
    println!("{:?}", NonNull::new(ptr::null_mut::<u8>()).is_none());
    let dangling: NonNull<u64> = NonNull::dangling();
    println!("dangling aligned: {}", dangling.as_ptr() as usize % 8 == 0);

    let bytes = 0x01020304u32.to_ne_bytes();
    let w = unsafe { (bytes.as_ptr() as *const u32).read_unaligned() };
    println!("{:#x}", w);
    let addr_diff = unsafe { (&v[3] as *const i32).offset_from(&v[0] as *const i32) };
    println!("{}", addr_diff);
}
