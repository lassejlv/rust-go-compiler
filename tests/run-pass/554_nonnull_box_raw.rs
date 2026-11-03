// NonNull, Box::into_raw/from_raw, a hand-written doubly linked list with raw pointers, Rc::into_raw
use std::ptr::NonNull;
use std::rc::Rc;

struct Node<T> {
    val: T,
    prev: Option<NonNull<Node<T>>>,
    next: Option<NonNull<Node<T>>>,
}

struct DList<T> {
    head: Option<NonNull<Node<T>>>,
    tail: Option<NonNull<Node<T>>>,
    len: usize,
}

impl<T: std::fmt::Debug> DList<T> {
    fn new() -> Self {
        DList { head: None, tail: None, len: 0 }
    }
    fn push_back(&mut self, v: T) {
        let node = Box::new(Node { val: v, prev: self.tail, next: None });
        let nn = NonNull::new(Box::into_raw(node)).unwrap();
        match self.tail {
            Some(mut t) => unsafe { t.as_mut().next = Some(nn) },
            None => self.head = Some(nn),
        }
        self.tail = Some(nn);
        self.len += 1;
    }
    fn push_front(&mut self, v: T) {
        let nn = NonNull::from(Box::leak(Box::new(Node { val: v, prev: None, next: self.head })));
        match self.head {
            Some(h) => unsafe { (*h.as_ptr()).prev = Some(nn) },
            None => self.tail = Some(nn),
        }
        self.head = Some(nn);
        self.len += 1;
    }
    fn pop_front(&mut self) -> Option<T> {
        self.head.map(|h| unsafe {
            let b = Box::from_raw(h.as_ptr());
            self.head = b.next;
            match self.head {
                Some(mut n) => n.as_mut().prev = None,
                None => self.tail = None,
            }
            self.len -= 1;
            b.val
        })
    }
    fn pop_back(&mut self) -> Option<T> {
        self.tail.map(|t| unsafe {
            let b = Box::from_raw(t.as_ptr());
            self.tail = b.prev;
            match self.tail {
                Some(mut n) => n.as_mut().next = None,
                None => self.head = None,
            }
            self.len -= 1;
            b.val
        })
    }
    fn to_vec_rev(&self) -> Vec<&T> {
        let mut out = vec![];
        let mut cur = self.tail;
        while let Some(n) = cur {
            unsafe {
                out.push(&(*n.as_ptr()).val);
                cur = n.as_ref().prev;
            }
        }
        out
    }
    fn for_each_mut(&mut self, mut f: impl FnMut(&mut T)) {
        let mut cur = self.head;
        while let Some(mut n) = cur {
            unsafe {
                f(&mut n.as_mut().val);
                cur = n.as_ref().next;
            }
        }
    }
}
impl<T> Drop for DList<T> {
    fn drop(&mut self) {
        let mut cur = self.head;
        let mut n = 0;
        while let Some(p) = cur {
            let b = unsafe { Box::from_raw(p.as_ptr()) };
            cur = b.next;
            n += 1;
        }
        println!("DList dropped {} nodes", n);
    }
}

#[derive(Debug)]
struct Loud(i32);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("Loud({}) dropped", self.0);
    }
}

fn main() {
    let mut l = DList::new();
    for i in 1..=4 {
        l.push_back(i * 10);
    }
    l.push_front(5);
    l.push_front(1);
    println!("len {} rev {:?}", l.len, l.to_vec_rev());
    println!("{:?} {:?}", l.pop_front(), l.pop_back());
    l.for_each_mut(|v| *v += 1);
    println!("{:?}", l.to_vec_rev());
    while let Some(v) = l.pop_back() {
        print!("{} ", v);
    }
    println!("| len {}", l.len);
    l.push_back(99);
    println!("{:?}", l.to_vec_rev());
    drop(l);
    let mut ll = DList::new();
    ll.push_back(Loud(1));
    ll.push_back(Loud(2));
    ll.push_front(Loud(0));
    let p = ll.pop_front();
    println!("popped {:?}", p);
    drop(p);
    drop(ll);
    let b = Box::new(String::from("raw"));
    let raw = Box::into_raw(b);
    unsafe {
        (*raw).push_str(" string");
        let back = Box::from_raw(raw);
        println!("{}", back);
    }
    let mut x = 5;
    let mut nn = NonNull::from(&mut x);
    unsafe {
        *nn.as_mut() += 1;
        *nn.as_ptr() *= 10;
    }
    println!("{}", x);
    println!("{:?}", NonNull::new(std::ptr::null_mut::<u8>()).is_none());
    let dangling: NonNull<u64> = NonNull::dangling();
    println!("{}", dangling.as_ptr() as usize % std::mem::align_of::<u64>());
    println!("{}", std::mem::size_of::<Option<NonNull<u8>>>() == std::mem::size_of::<*mut u8>());
    let rc = Rc::new(Loud(7));
    let rc2 = rc.clone();
    let rp = Rc::into_raw(rc);
    println!("count {}", Rc::strong_count(&rc2));
    let rc3 = unsafe { Rc::from_raw(rp) };
    println!("value {} count {}", rc3.0, Rc::strong_count(&rc3));
    drop(rc3);
    println!("count {}", Rc::strong_count(&rc2));
    let arr = Box::new([1u16, 2, 3]);
    let ap: *mut [u16; 3] = Box::into_raw(arr);
    let sl: Box<[u16]> = unsafe { Box::from_raw(ap as *mut [u16; 3]) };
    println!("{:?}", sl);
    println!("end");
}
