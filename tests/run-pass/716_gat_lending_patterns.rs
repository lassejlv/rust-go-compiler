// Generic associated types: lending iterators over windows of a mutable
// buffer, a GAT-based collection family trait, and a pointer-family trait.
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::Arc;
use std::ops::Deref;

trait LendingIterator {
    type Item<'a> where Self: 'a;
    fn next<'a>(&'a mut self) -> Option<Self::Item<'a>>;

    fn count_all(mut self) -> usize where Self: Sized {
        let mut n = 0;
        while self.next().is_some() { n += 1; }
        n
    }
}

struct WindowsMut<'s, T> { data: &'s mut [T], pos: usize, size: usize }

impl<'s, T> LendingIterator for WindowsMut<'s, T> {
    type Item<'a> = &'a mut [T] where Self: 'a;
    fn next<'a>(&'a mut self) -> Option<&'a mut [T]> {
        if self.pos + self.size > self.data.len() { return None; }
        let w = &mut self.data[self.pos..self.pos + self.size];
        self.pos += 1;
        Some(w)
    }
}

struct LineBuf { text: String, buf: String, pos: usize }

impl LendingIterator for LineBuf {
    type Item<'a> = (usize, &'a str);
    fn next<'a>(&'a mut self) -> Option<(usize, &'a str)> {
        if self.pos >= self.text.len() { return None; }
        let rest = &self.text[self.pos..];
        let end = rest.find('\n').unwrap_or(rest.len());
        self.buf.clear();
        self.buf.push_str(rest[..end].trim());
        self.buf.make_ascii_uppercase();
        let line_start = self.pos;
        self.pos += end + 1;
        Some((line_start, &self.buf))
    }
}

fn for_each_lend<I: LendingIterator, F>(mut it: I, mut f: F) where F: for<'a> FnMut(I::Item<'a>) {
    while let Some(x) = it.next() { f(x); }
}

trait Family {
    type Coll<T>;
    fn empty<T>() -> Self::Coll<T>;
    fn push<T>(c: &mut Self::Coll<T>, v: T);
    fn items<T: Clone>(c: &Self::Coll<T>) -> Vec<T>;
    const NAME: &'static str;
}

struct VecFam;
struct DequeFrontFam;

impl Family for VecFam {
    type Coll<T> = Vec<T>;
    fn empty<T>() -> Vec<T> { Vec::new() }
    fn push<T>(c: &mut Vec<T>, v: T) { c.push(v) }
    fn items<T: Clone>(c: &Vec<T>) -> Vec<T> { c.clone() }
    const NAME: &'static str = "vec";
}
impl Family for DequeFrontFam {
    type Coll<T> = VecDeque<T>;
    fn empty<T>() -> VecDeque<T> { VecDeque::new() }
    fn push<T>(c: &mut VecDeque<T>, v: T) { c.push_front(v) }
    fn items<T: Clone>(c: &VecDeque<T>) -> Vec<T> { c.iter().cloned().collect() }
    const NAME: &'static str = "deque-front";
}

fn build<F: Family>(n: u32) -> (Vec<u32>, Vec<String>) {
    let mut a: F::Coll<u32> = F::empty();
    let mut b: F::Coll<String> = F::empty();
    for i in 0..n {
        F::push(&mut a, i * i);
        F::push(&mut b, format!("{}{}", F::NAME.chars().next().unwrap(), i));
    }
    (F::items(&a), F::items(&b))
}

trait PointerFamily {
    type Ptr<T>: Deref<Target = T> + Clone;
    fn new<T>(v: T) -> Self::Ptr<T>;
}
struct RcFam;
struct ArcFam;
impl PointerFamily for RcFam { type Ptr<T> = Rc<T>; fn new<T>(v: T) -> Rc<T> { Rc::new(v) } }
impl PointerFamily for ArcFam { type Ptr<T> = Arc<T>; fn new<T>(v: T) -> Arc<T> { Arc::new(v) } }

struct Tree<P: PointerFamily> { val: i32, kids: Vec<P::Ptr<Tree<P>>> }

impl<P: PointerFamily> Tree<P> {
    fn leaf(v: i32) -> P::Ptr<Tree<P>> { P::new(Tree { val: v, kids: vec![] }) }
    fn sum(&self) -> i32 { self.val + self.kids.iter().map(|k| k.sum()).sum::<i32>() }
}

fn main() {
    let mut data = [1, 2, 3, 4, 5, 6];
    let mut w = WindowsMut { data: &mut data, pos: 0, size: 3 };
    while let Some(win) = w.next() {
        win[2] += win[0] * win[1];
        win.reverse();
    }
    println!("windows result: {:?}", data);
    let n = WindowsMut { data: &mut [0u8; 10], pos: 0, size: 4 }.count_all();
    println!("window count: {}", n);

    let lb = LineBuf { text: "  alpha\nbeta  \n\ngamma delta\nz".to_string(), buf: String::new(), pos: 0 };
    let mut seen = Vec::new();
    for_each_lend(lb, |(off, line)| seen.push(format!("{}:{}", off, line)));
    println!("lines: {:?}", seen);

    println!("{:?}", build::<VecFam>(4));
    println!("{:?}", build::<DequeFrontFam>(4));

    let shared = Tree::<RcFam>::leaf(10);
    let t: Tree<RcFam> = Tree { val: 1, kids: vec![Tree::<RcFam>::leaf(2), shared.clone(), shared.clone()] };
    println!("rc tree sum={} shared strong={}", t.sum(), Rc::strong_count(&shared));
    let at: Tree<ArcFam> = Tree { val: 5, kids: vec![Arc::new(Tree { val: 6, kids: vec![Tree::<ArcFam>::leaf(7)] })] };
    println!("arc tree sum={}", at.sum());
}
