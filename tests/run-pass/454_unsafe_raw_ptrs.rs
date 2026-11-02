// Unsafe raw pointer code: pointer arithmetic (add/offset/sub/offset_from), read/write,
// ptr::copy (overlapping) vs copy_nonoverlapping, ptr::swap, slice::from_raw_parts(_mut),
// a hand-rolled singly linked list with Box::into_raw/from_raw, null checks, and addr_of!.
use std::ptr;

struct RawList {
    head: *mut RawNode,
    len: usize,
}
struct RawNode {
    val: i32,
    next: *mut RawNode,
}

impl RawList {
    fn new() -> Self {
        RawList { head: ptr::null_mut(), len: 0 }
    }
    fn push_front(&mut self, v: i32) {
        let n = Box::into_raw(Box::new(RawNode { val: v, next: self.head }));
        self.head = n;
        self.len += 1;
    }
    fn reverse(&mut self) {
        let mut prev: *mut RawNode = ptr::null_mut();
        let mut cur = self.head;
        unsafe {
            while !cur.is_null() {
                let next = (*cur).next;
                (*cur).next = prev;
                prev = cur;
                cur = next;
            }
        }
        self.head = prev;
    }
    fn to_vec(&self) -> Vec<i32> {
        let mut v = vec![];
        let mut cur = self.head;
        while let Some(n) = unsafe { cur.as_ref() } {
            v.push(n.val);
            cur = n.next;
        }
        v
    }
    fn remove_if(&mut self, f: impl Fn(i32) -> bool) -> usize {
        let mut removed = 0;
        let mut link: *mut *mut RawNode = &mut self.head;
        unsafe {
            while !(*link).is_null() {
                let node = *link;
                if f((*node).val) {
                    *link = (*node).next;
                    drop(Box::from_raw(node));
                    removed += 1;
                    self.len -= 1;
                } else {
                    link = &mut (*node).next;
                }
            }
        }
        removed
    }
}
impl Drop for RawList {
    fn drop(&mut self) {
        let mut cur = self.head;
        let mut n = 0;
        while !cur.is_null() {
            let b = unsafe { Box::from_raw(cur) };
            cur = b.next;
            n += 1;
        }
        println!("freed {} nodes", n);
    }
}

#[repr(C)]
struct Header {
    tag: u8,
    flags: u8,
    len: u16,
    id: u32,
}

fn main() {
    let mut arr = [1, 2, 3, 4, 5, 6, 7, 8];
    let p = arr.as_mut_ptr();
    unsafe {
        *p.add(2) = 30;
        let q = p.add(7);
        *q.sub(1) += 100;
        println!("offset_from {}", q.offset_from(p));
        println!("read {} {}", ptr::read(p.add(1)), *p.offset(4));
        ptr::write(p, -1);
        // overlapping copy: shift elements right by 2
        ptr::copy(p, p.add(2), 4);
    }
    println!("{:?}", arr);

    let src = [9u8, 8, 7];
    let mut dst = [0u8; 5];
    unsafe {
        ptr::copy_nonoverlapping(src.as_ptr(), dst.as_mut_ptr().add(1), src.len());
    }
    println!("{:?}", dst);

    let mut a = String::from("left");
    let mut b = String::from("right");
    unsafe { ptr::swap(&mut a, &mut b) };
    println!("{} {}", a, b);
    let old = unsafe { ptr::replace(&mut a, String::from("new")) };
    println!("{} {}", old, a);

    let v = vec![10u16, 20, 30, 40, 50];
    let s = unsafe { std::slice::from_raw_parts(v.as_ptr().add(1), 3) };
    println!("{:?}", s);
    let mut w = vec![1i64; 6];
    let sm = unsafe { std::slice::from_raw_parts_mut(w.as_mut_ptr().add(2), 2) };
    sm[0] = 7;
    sm[1] = 8;
    println!("{:?}", w);

    let mut list = RawList::new();
    for i in 1..=8 {
        list.push_front(i * i);
    }
    println!("{:?} len={}", list.to_vec(), list.len);
    list.reverse();
    println!("{:?}", list.to_vec());
    let r = list.remove_if(|v| v % 2 == 0);
    println!("removed {} -> {:?} len={}", r, list.to_vec(), list.len);

    let h = Header { tag: 1, flags: 0x80, len: 513, id: 0xdeadbeef };
    let base = &h as *const Header as *const u8;
    unsafe {
        let len_ptr = ptr::addr_of!(h.len) as *const u8;
        let id_ptr = ptr::addr_of!(h.id) as *const u8;
        println!("offsets len={} id={} size={}", len_ptr.offset_from(base), id_ptr.offset_from(base), std::mem::size_of::<Header>());
        println!("tag={} flags={:#x} len={} id={:#x}", *base, *base.add(1), (len_ptr as *const u16).read_unaligned(), (id_ptr as *const u32).read());
    }

    let np: *const i32 = ptr::null();
    println!("null {} {:?}", np.is_null(), unsafe { np.as_ref() });
    let x = 5;
    let px = &x as *const i32;
    let addr_eq = px == &x as *const i32;
    println!("eq {} {}", addr_eq, unsafe { *px } + 1);
    let boxed = Box::new([1u32, 2, 3]);
    let raw = Box::into_raw(boxed);
    unsafe {
        (*raw)[1] = 20;
        let back = Box::from_raw(raw);
        println!("{:?}", back);
    }
}
