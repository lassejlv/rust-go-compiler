use std::ptr::NonNull;

struct Node<T> {
    val: T,
    prev: Option<NonNull<Node<T>>>,
    next: Option<NonNull<Node<T>>>,
}

pub struct DList<T> {
    head: Option<NonNull<Node<T>>>,
    tail: Option<NonNull<Node<T>>>,
    len: usize,
}

impl<T: std::fmt::Debug> DList<T> {
    fn new() -> Self { DList { head: None, tail: None, len: 0 } }
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
        while let Some(p) = cur {
            unsafe {
                out.push(&(*p.as_ptr()).val);
                cur = (*p.as_ptr()).prev;
            }
        }
        out
    }
    fn iter(&self) -> Iter<'_, T> {
        Iter { cur: self.head, _m: std::marker::PhantomData }
    }
}

struct Iter<'a, T> {
    cur: Option<NonNull<Node<T>>>,
    _m: std::marker::PhantomData<&'a T>,
}
impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<&'a T> {
        self.cur.map(|p| unsafe {
            let r = &*p.as_ptr();
            self.cur = r.next;
            &r.val
        })
    }
}

impl<T> Drop for DList<T> {
    fn drop(&mut self) {
        let mut cur = self.head;
        let mut n = 0;
        while let Some(p) = cur {
            unsafe {
                let b = Box::from_raw(p.as_ptr());
                cur = b.next;
            }
            n += 1;
        }
        println!("freed {} nodes", n);
    }
}

#[derive(Debug)]
struct D(i32);
impl Drop for D {
    fn drop(&mut self) { println!("drop D{}", self.0); }
}

fn main() {
    let mut l = DList::new();
    for i in 1..=4 { l.push_back(i * 10); }
    l.push_front(5);
    println!("{:?}", l.iter().collect::<Vec<_>>());
    println!("{:?}", l.to_vec_rev());
    println!("{:?} {:?}", l.pop_front(), l.pop_back());
    println!("len {} sum {}", l.len, l.iter().sum::<i32>());
    {
        let mut l2 = DList::new();
        l2.push_back(D(1));
        l2.push_back(D(2));
        l2.push_back(D(3));
        let x = l2.pop_front();
        println!("popped {:?}", x);
    }
    let mut x = 5;
    let nn = NonNull::from(&mut x);
    unsafe { *nn.as_ptr() += 1; }
    println!("x={} dangling_is_aligned={}", x, NonNull::<u64>::dangling().as_ptr() as usize % 8 == 0);
}
