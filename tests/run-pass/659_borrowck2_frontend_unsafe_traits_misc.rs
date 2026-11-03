// Valid programs that a naive borrow checker / front end might reject.
// Each module is an independent program; main runs them in order.

mod p24_unsafe_blocks {
    static mut COUNTER: u32 = 0;
    unsafe fn bump(n: u32) -> u32 { unsafe { COUNTER += n; COUNTER } }
    unsafe trait Zeroable { fn zero() -> Self; }
    unsafe impl Zeroable for u64 { fn zero() -> Self { 0 } }
    union Bits { f: f32, u: u32 }
    fn raw_sum(v: &[i32]) -> i32 {
        let p = v.as_ptr();
        let mut s = 0;
        for i in 0..v.len() { s += unsafe { *p.add(i) }; }
        s
    }
    pub fn main() {
        let mut x = 5;
        let p = &mut x as *mut i32;
        unsafe { *p += 1; *p *= 2; }
        let r = unsafe { bump(3); bump(4) };
        let b = Bits { f: 1.0 };
        let u = unsafe { b.u };
        let total = unsafe { *std::ptr::addr_of!(COUNTER) };
        let mut arr = [1, 2, 3];
        let q = arr.as_mut_ptr();
        unsafe { std::ptr::swap(q, q.add(2)); }
        println!("{} {} {:#x} {} {} {:?} {}", x, r, u, total, raw_sum(&[1, 2, 3]), arr, <u64 as Zeroable>::zero());
    }
}

mod p38_trait_generic_dispatch {
    use std::fmt::Display;
    use std::ops::Add;
    trait Shape { fn area(&self) -> f64; fn name(&self) -> String where Self: Sized { "shape".into() } fn scaled(&self, k: f64) -> f64 { self.area() * k } }
    struct Sq(f64);
    impl Shape for Sq { fn area(&self) -> f64 { self.0 * self.0 } }
    #[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
    struct V(i32, i32);
    impl Add for V { type Output = V; fn add(self, o: V) -> V { V(self.0 + o.0, self.1 + o.1) } }
    fn sum_all<T: Add<Output = T> + Default + Copy>(xs: &[T]) -> T { xs.iter().fold(T::default(), |a, &b| a + b) }
    fn show<T>(t: T) -> String where T: Display { format!("[{}]", t) }
    trait Limits { const MAX: u8; }
    struct L;
    impl Limits for L { const MAX: u8 = 9; }
    impl L { const MIN: u8 = 1; fn check(n: u8) -> &'static str { match n { <L as Limits>::MAX => "max", L::MIN => "min", _ => "other" } } }
    pub fn main() {
        let shapes: Vec<Box<dyn Shape>> = vec![Box::new(Sq(2.0)), Box::new(Sq(3.0))];
        let tot: f64 = shapes.iter().map(|s| s.scaled(0.5)).sum();
        let s = Sq(1.0);
        let v = sum_all(&[V(1, 2), V(3, 4)]);
        println!("{} {} {:?} {} {} {} {}", tot, s.name(), v, V(1, 2) < V(1, 3), show(4.5), L::check(9), L::check(1));
    }
}

mod p40_deref_coercions_generics {
    use std::collections::BTreeMap;
    use std::rc::Rc;
    use std::cell::RefCell;
    fn takes_str(s: &str) -> usize { s.len() }
    fn takes_slice(s: &[i32]) -> i32 { s.iter().sum() }
    struct Wrapper { inner: Vec<i32> }
    impl std::ops::Deref for Wrapper { type Target = Vec<i32>; fn deref(&self) -> &Vec<i32> { &self.inner } }
    impl std::ops::DerefMut for Wrapper { fn deref_mut(&mut self) -> &mut Vec<i32> { &mut self.inner } }
    pub fn main() {
        let b: Box<String> = Box::new("boxed".into());
        let rc: Rc<String> = Rc::new("rc".into());
        let mut w = Wrapper { inner: vec![1, 2] };
        w.push(3);
        let total = takes_slice(&w);
        let shared = Rc::new(RefCell::new(BTreeMap::new()));
        let s2 = Rc::clone(&shared);
        s2.borrow_mut().insert("k", 1);
        {
            let mut m = shared.borrow_mut();
            *m.entry("k").or_insert(0) += 10;
            m.insert("j", 2);
        }
        let snapshot: Vec<_> = shared.borrow().iter().map(|(k, v)| format!("{}{}", k, v)).collect();
        println!("{} {} {} {:?} {}", takes_str(&b), takes_str(&rc), total, snapshot, Rc::strong_count(&shared));
    }
}

mod p52_gat_lending {
    trait Lender { type Item<'a> where Self: 'a; fn next<'a>(&'a mut self) -> Option<Self::Item<'a>>; }
    struct Windows { v: Vec<i32>, pos: usize }
    impl Lender for Windows {
        type Item<'a> = &'a mut [i32];
        fn next<'a>(&'a mut self) -> Option<&'a mut [i32]> {
            if self.pos + 2 > self.v.len() { return None; }
            let s = &mut self.v[self.pos..self.pos + 2];
            self.pos += 1;
            Some(s)
        }
    }
    pub fn main() {
        let mut w = Windows { v: vec![1, 2, 3, 4], pos: 0 };
        while let Some(win) = w.next() { win[1] += win[0]; }
        println!("{:?}", w.v);
    }
}

mod p55_rc_refcell_graph {
    use std::cell::RefCell;
    use std::rc::{Rc, Weak};
    #[derive(Debug)]
    struct Node { val: i32, children: RefCell<Vec<Rc<Node>>>, parent: RefCell<Weak<Node>> }
    pub fn main() {
        let leaf = Rc::new(Node { val: 3, children: RefCell::new(vec![]), parent: RefCell::new(Weak::new()) });
        let branch = Rc::new(Node { val: 5, children: RefCell::new(vec![Rc::clone(&leaf)]), parent: RefCell::new(Weak::new()) });
        *leaf.parent.borrow_mut() = Rc::downgrade(&branch);
        let pv = leaf.parent.borrow().upgrade().map(|p| p.val);
        let sum: i32 = branch.children.borrow().iter().map(|c| c.val).sum();
        {
            let mut ch = branch.children.borrow_mut();
            ch.push(Rc::new(Node { val: 7, children: RefCell::new(vec![]), parent: RefCell::new(Weak::new()) }));
        }
        println!("{:?} {} {} {} {}", pv, sum, branch.children.borrow().len(), Rc::strong_count(&leaf), Rc::weak_count(&branch));
    }
}

mod p60_numeric_inference_edge {
    pub fn main() {
        let a = 255u8 as i8;
        let b = -1i32 as u32;
        let c = 3.9f64 as i32;
        let d = (1u64 << 40) as u32;
        let e = i32::MAX.wrapping_add(1);
        let f = 7 / -2;
        let g = 7 % -2;
        let h = (-7i32).rem_euclid(3);
        let i = 2i32.pow(10) + 0x10 + 0o7 + 0b11;
        let j = 1_000_000i64 * 1_000_000;
        let k: u8 = b'a' + 1;
        let l = 'a' as u8 as char;
        let m = (300i32).clamp(0, 255) as u8;
        let n = u32::from_str_radix("ff", 16).unwrap();
        let o = 0.1 + 0.2;
        let p = f32::MAX > 1e38;
        let q = (-8.0f64).abs().sqrt();
        let checked = 250u8.checked_add(10);
        let (r, ov) = 250u8.overflowing_add(10);
        println!("{} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {:?} {} {}", a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, checked, r, ov);
    }
}

fn main() {
    p24_unsafe_blocks::main();
    p38_trait_generic_dispatch::main();
    p40_deref_coercions_generics::main();
    p52_gat_lending::main();
    p55_rc_refcell_graph::main();
    p60_numeric_inference_edge::main();
}
