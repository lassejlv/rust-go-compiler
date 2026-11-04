mod w02 {
    use std::ptr::{self, NonNull};
    use std::mem::MaybeUninit;
    struct Node { v: i32, next: *mut Node }
    struct Queue { head: *mut Node, tail: *mut Node, len: usize }
    impl Queue {
        fn new() -> Self { Queue { head: ptr::null_mut(), tail: ptr::null_mut(), len: 0 } }
        fn push(&mut self, v: i32) {
            let n = Box::into_raw(Box::new(Node { v, next: ptr::null_mut() }));
            if self.tail.is_null() { self.head = n; } else { unsafe { (*self.tail).next = n; } }
            self.tail = n; self.len += 1;
        }
        fn pop(&mut self) -> Option<i32> {
            if self.head.is_null() { return None; }
            let b = unsafe { Box::from_raw(self.head) };
            self.head = b.next;
            if self.head.is_null() { self.tail = ptr::null_mut(); }
            self.len -= 1;
            Some(b.v)
        }
    }
    impl Drop for Queue { fn drop(&mut self) { let mut c = 0; while self.pop().is_some() { c += 1; } println!("drained {}", c); } }
    #[repr(C)] union IntOrFloat { i: u32, f: f32 }
    pub fn main() {
        let mut q = Queue::new();
        for i in 1..=5 { q.push(i * i); }
        println!("{:?} {:?} {}", q.pop(), q.pop(), q.len);
        let mut arr = [1, 2, 3, 4];
        let p = arr.as_mut_ptr();
        unsafe {
            *p.add(2) = 30;
            ptr::swap(p, p.add(3));
            let s = std::slice::from_raw_parts(p.add(1), 3);
            println!("{:?} {}", s, p.add(3).read());
            p.write(100);
            println!("{}", *p.offset(0) + *p.wrapping_add(1));
        }
        println!("{:?}", arr);
        let mut m: MaybeUninit<[u8; 4]> = MaybeUninit::uninit();
        unsafe { m.as_mut_ptr().write([9, 8, 7, 6]); }
        let m = unsafe { m.assume_init() };
        println!("{:?}", m);
        let u = IntOrFloat { f: 1.0 };
        println!("{:#x}", unsafe { u.i });
        let bits: u32 = unsafe { std::mem::transmute(2.0f32) };
        let back: f32 = f32::from_bits(bits);
        println!("{:#x} {}", bits, back);
        let x = 5i32;
        let nn = NonNull::from(&x);
        println!("{}", unsafe { *nn.as_ptr() });
        let v = vec![1u16, 2, 3];
        let mut v = std::mem::ManuallyDrop::new(v);
        let (p, l, c) = (v.as_mut_ptr(), v.len(), v.capacity());
        let rebuilt = unsafe { Vec::from_raw_parts(p, l, c) };
        println!("{:?}", rebuilt);
        let s = String::from("héllo");
        let bytes = s.as_bytes();
        println!("{}", unsafe { std::str::from_utf8_unchecked(&bytes[0..3]) });
        let addr_eq = ptr::eq(&arr[0], &arr[0]);
        println!("{} {}", addr_eq, std::mem::size_of::<*const u8>());
        let raw: *const [i32] = &arr[..];
        println!("{}", raw.len());
        println!("{}", std::mem::align_of::<u64>());
    }
}

mod w04 {
    use std::fmt::Debug;
    fn print_all<C, T: Debug>(c: &C) where for<'a> &'a C: IntoIterator<Item = &'a T> {
        for x in c { print!("{:?} ", x); }
        println!();
    }
    fn apply_all<F>(fs: &[F], x: i32) -> Vec<i32> where F: Fn(i32) -> i32 { fs.iter().map(|f| f(x)).collect() }
    fn twice<T, F: FnOnce(T) -> T + Copy>(f: F, x: T) -> T { f(f(x)) }
    fn mutual_even(n: u32) -> bool { if n == 0 { true } else { mutual_odd(n - 1) } }
    fn mutual_odd(n: u32) -> bool { if n == 0 { false } else { mutual_even(n - 1) } }
    fn gen_outer<T: Debug + Clone>(x: T) -> String {
        fn inner<U: Debug>(u: U, n: usize) -> String { format!("{:?}x{}", u, n) }
        inner(vec![x.clone(), x], 2)
    }
    struct Parser<'a> { s: &'a [u8], pos: usize }
    impl<'a> Parser<'a> {
        fn next_tok(&mut self) -> Option<&'a [u8]> {
            while self.pos < self.s.len() && self.s[self.pos] == b' ' { self.pos += 1; }
            let st = self.pos;
            while self.pos < self.s.len() && self.s[self.pos] != b' ' { self.pos += 1; }
            if st == self.pos { None } else { Some(&self.s[st..self.pos]) }
        }
    }
    fn collect_toks<'a>(p: &mut Parser<'a>) -> Vec<&'a str> {
        let mut v = vec![];
        while let Some(t) = p.next_tok() { v.push(std::str::from_utf8(t).unwrap()); }
        v
    }
    pub fn main() {
        print_all(&vec![1, 2, 3]);
        print_all(&std::collections::BTreeSet::from(["b", "a"]));
        let fs = [|x| x + 1, |x| x * 2];
        println!("{:?}", apply_all(&fs, 5));
        println!("{}", twice(|s: u64| s * 3, 7));
        println!("{} {}", mutual_even(10), mutual_odd(7));
        println!("{}", gen_outer('z'));
        let src = String::from("  let x =  42 ;");
        let mut p = Parser { s: src.as_bytes(), pos: 0 };
        let toks = collect_toks(&mut p);
        println!("{:?}", toks);
        println!("{} {}", 2.0_f64.sqrt() > 1.41, 9f32.sqrt());
        let mut counter = 0;
        let mut inc = || { let mut inner = || counter += 1; inner(); inner(); };
        inc(); inc();
        println!("{}", counter);
        let mut data = vec![1, 2, 3];
        let mut modify = |i: usize| { let get = |d: &mut Vec<i32>, j: usize| d[j] *= 10; get(&mut data, i); };
        modify(0); modify(2);
        println!("{:?}", data);
    }
}

fn main() {
    println!("== w02");
    w02::main();
    println!("== w04");
    w04::main();
}
