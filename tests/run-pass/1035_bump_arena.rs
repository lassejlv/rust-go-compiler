use std::alloc::{alloc, dealloc, Layout};
use std::cell::{Cell, RefCell};
use std::marker::PhantomData;

struct Arena {
    chunks: RefCell<Vec<(*mut u8, usize)>>,
    cur: Cell<*mut u8>,
    end: Cell<*mut u8>,
    allocated: Cell<usize>,
}

impl Arena {
    const CHUNK: usize = 64;
    fn new() -> Self {
        Arena { chunks: RefCell::new(vec![]), cur: Cell::new(std::ptr::null_mut()), end: Cell::new(std::ptr::null_mut()), allocated: Cell::new(0) }
    }
    fn new_chunk(&self, min: usize) {
        let size = Self::CHUNK.max(min);
        let p = unsafe { alloc(Layout::from_size_align(size, 16).unwrap()) };
        self.chunks.borrow_mut().push((p, size));
        self.cur.set(p);
        self.end.set(unsafe { p.add(size) });
    }
    fn alloc<T>(&self, v: T) -> &mut T {
        let l = Layout::new::<T>();
        assert!(l.align() <= 16);
        loop {
            let cur = self.cur.get() as usize;
            let aligned = (cur + l.align() - 1) & !(l.align() - 1);
            if !self.cur.get().is_null() && aligned + l.size() <= self.end.get() as usize {
                let p = aligned as *mut T;
                self.cur.set((aligned + l.size()) as *mut u8);
                self.allocated.set(self.allocated.get() + l.size());
                unsafe {
                    p.write(v);
                    return &mut *p;
                }
            }
            self.new_chunk(l.size() + l.align());
        }
    }
    fn alloc_slice_copy<T: Copy>(&self, s: &[T]) -> &mut [T] {
        let first = self.alloc(s[0]) as *mut T;
        for x in &s[1..] { self.alloc(*x); }
        unsafe { std::slice::from_raw_parts_mut(first, s.len()) }
    }
}

impl Drop for Arena {
    fn drop(&mut self) {
        let chunks = self.chunks.borrow();
        for &(p, size) in chunks.iter() {
            unsafe { dealloc(p, Layout::from_size_align(size, 16).unwrap()); }
        }
        println!("arena freed {} chunks, {} bytes used", chunks.len(), self.allocated.get());
    }
}

#[derive(Debug)]
enum Expr<'a> {
    Num(i64),
    Add(&'a Expr<'a>, &'a Expr<'a>),
    Mul(&'a Expr<'a>, &'a Expr<'a>),
    Neg(&'a Expr<'a>),
}

fn eval(e: &Expr) -> i64 {
    match e {
        Expr::Num(n) => *n,
        Expr::Add(a, b) => eval(a) + eval(b),
        Expr::Mul(a, b) => eval(a) * eval(b),
        Expr::Neg(a) => -eval(a),
    }
}

struct Id<'a>(u32, PhantomData<&'a ()>);

fn main() {
    let arena = Arena::new();
    let a = arena.alloc(1u8);
    let b = arena.alloc(0x1122334455667788u64);
    let c = arena.alloc([1u16, 2, 3]);
    *a += 1;
    c[2] = 99;
    println!("{} {:#x} {:?}", a, b, c);
    println!("u64 aligned {}", (b as *mut u64 as usize) % 8 == 0);
    let one = arena.alloc(Expr::Num(1));
    let two = arena.alloc(Expr::Num(2));
    let five = arena.alloc(Expr::Num(5));
    let sum = arena.alloc(Expr::Add(one, two));
    let prod = arena.alloc(Expr::Mul(sum, five));
    let neg = arena.alloc(Expr::Neg(prod));
    println!("{:?} = {}", neg, eval(neg));
    let mut nodes: Vec<&Expr> = vec![];
    for i in 0..20 {
        nodes.push(arena.alloc(Expr::Num(i)));
    }
    let mut acc: &Expr = arena.alloc(Expr::Num(0));
    for n in nodes { acc = arena.alloc(Expr::Add(acc, n)); }
    println!("sum 0..20 = {}", eval(acc));
    let s = arena.alloc_slice_copy(&[10i32, 20, 30]);
    s.reverse();
    println!("{:?}", s);
    let big = arena.alloc([7u8; 100]);
    println!("big {} {}", big.len(), big.iter().map(|&x| x as u32).sum::<u32>());
    let id = arena.alloc(Id(5, PhantomData));
    println!("id {}", id.0);
}
