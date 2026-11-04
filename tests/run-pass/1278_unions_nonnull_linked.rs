// Unions, NonNull-based doubly linked list, Box::into_raw/from_raw, ManuallyDrop in unions.
use std::mem::ManuallyDrop;
use std::ptr::NonNull;

#[repr(C)]
union IntOrFloat {
    i: u32,
    f: f32,
    bytes: [u8; 4],
}

union Slot {
    num: u64,
    text: ManuallyDrop<String>,
}

struct Tagged {
    is_text: bool,
    slot: Slot,
}
impl Drop for Tagged {
    fn drop(&mut self) {
        if self.is_text {
            unsafe { ManuallyDrop::drop(&mut self.slot.text) };
            println!("dropped text slot");
        }
    }
}

struct Node<T> {
    val: T,
    prev: Option<NonNull<Node<T>>>,
    next: Option<NonNull<Node<T>>>,
}

struct List<T> {
    head: Option<NonNull<Node<T>>>,
    tail: Option<NonNull<Node<T>>>,
    len: usize,
}

impl<T: std::fmt::Debug> List<T> {
    fn new() -> Self {
        List { head: None, tail: None, len: 0 }
    }
    fn push_back(&mut self, val: T) {
        let node = Box::new(Node { val, prev: self.tail, next: None });
        let ptr = NonNull::new(Box::into_raw(node)).unwrap();
        match self.tail {
            Some(mut t) => unsafe { t.as_mut().next = Some(ptr) },
            None => self.head = Some(ptr),
        }
        self.tail = Some(ptr);
        self.len += 1;
    }
    fn push_front(&mut self, val: T) {
        let node = Box::new(Node { val, prev: None, next: self.head });
        let ptr = unsafe { NonNull::new_unchecked(Box::into_raw(node)) };
        match self.head {
            Some(h) => unsafe { (*h.as_ptr()).prev = Some(ptr) },
            None => self.tail = Some(ptr),
        }
        self.head = Some(ptr);
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
}
impl<T> Drop for List<T> {
    fn drop(&mut self) {
        let mut cur = self.head;
        let mut n = 0;
        while let Some(p) = cur {
            let b = unsafe { Box::from_raw(p.as_ptr()) };
            cur = b.next;
            n += 1;
        }
        println!("list dropped {} nodes", n);
    }
}

fn main() {
    let mut u = IntOrFloat { f: 1.5 };
    unsafe {
        println!("{:#x} {:?}", u.i, u.bytes);
        u.i += 1;
        println!("{}", u.f > 1.5);
        u.bytes[3] = 0x40;
        println!("{}", u.f);
    }
    println!("union size {}", std::mem::size_of::<IntOrFloat>());
    let i = unsafe { let IntOrFloat { i } = IntOrFloat { i: 7 }; i };
    println!("i {}", i);
    let a = Tagged { is_text: false, slot: Slot { num: 77 } };
    let b = Tagged { is_text: true, slot: Slot { text: ManuallyDrop::new("hello".to_string()) } };
    unsafe {
        println!("{} {}", a.slot.num, &*b.slot.text);
        match a.slot {
            Slot { num: 77 } => println!("matched 77"),
            _ => println!("other"),
        }
    }
    drop(b);
    drop(a);

    let mut l = List::new();
    for i in 1..=4 {
        l.push_back(i * 10);
    }
    l.push_front(5);
    println!("len {} rev {:?}", l.len, l.to_vec_rev());
    println!("pop {:?} {:?}", l.pop_front(), l.pop_front());
    println!("rev {:?}", l.to_vec_rev());
    let mut ls = List::new();
    ls.push_back(String::from("x"));
    ls.push_back(String::from("y"));
    println!("{:?}", ls.pop_front());
    drop(ls);
    drop(l);

    let b = Box::new([1u16, 2, 3]);
    let raw = Box::into_raw(b);
    unsafe {
        (*raw)[1] = 20;
        let nn = NonNull::new(raw).unwrap();
        println!("{:?}", *nn.as_ptr());
        let b = Box::from_raw(raw);
        println!("{:?}", b);
    }
    let dangling: NonNull<u64> = NonNull::dangling();
    println!("dangling aligned {}", dangling.as_ptr() as usize % 8 == 0);
    let leaked: &'static mut Vec<i32> = Box::leak(Box::new(vec![1, 2]));
    leaked.push(3);
    println!("{:?}", leaked);
    unsafe { drop(Box::from_raw(leaked as *mut Vec<i32>)) };
}
