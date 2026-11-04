trait LendingIterator {
    type Item<'a> where Self: 'a;
    fn next<'a>(&'a mut self) -> Option<Self::Item<'a>>;
}

struct WindowsMut<'t, T> {
    slice: &'t mut [T],
    start: usize,
    size: usize,
}

impl<'t, T> LendingIterator for WindowsMut<'t, T> {
    type Item<'a> = &'a mut [T] where Self: 'a;
    fn next<'a>(&'a mut self) -> Option<Self::Item<'a>> {
        let r = self.start..self.start + self.size;
        if r.end > self.slice.len() {
            return None;
        }
        self.start += 1;
        Some(&mut self.slice[r])
    }
}

struct Lines {
    buf: String,
    data: Vec<&'static str>,
    idx: usize,
}

impl LendingIterator for Lines {
    type Item<'a> = &'a str;
    fn next<'a>(&'a mut self) -> Option<&'a str> {
        if self.idx >= self.data.len() {
            return None;
        }
        self.buf.clear();
        self.buf.push_str(self.data[self.idx]);
        self.buf.push('!');
        self.idx += 1;
        Some(&self.buf)
    }
}

fn count<L: LendingIterator>(mut l: L) -> usize {
    let mut n = 0;
    while let Some(_) = l.next() {
        n += 1;
    }
    n
}

fn for_each<L, F>(mut l: L, mut f: F)
where
    L: LendingIterator,
    F: for<'a> FnMut(L::Item<'a>),
{
    while let Some(x) = l.next() {
        f(x);
    }
}

trait Family {
    type Member<T>;
    fn wrap<T>(t: T) -> Self::Member<T>;
}
struct OptionFamily;
struct VecFamily;
impl Family for OptionFamily {
    type Member<T> = Option<T>;
    fn wrap<T>(t: T) -> Option<T> { Some(t) }
}
impl Family for VecFamily {
    type Member<T> = Vec<T>;
    fn wrap<T>(t: T) -> Vec<T> { vec![t] }
}
fn wrap_both<F: Family>(a: i32, b: &str) -> (F::Member<i32>, F::Member<String>) {
    (F::wrap(a), F::wrap(b.to_string()))
}

trait PointerFamily {
    type Ptr<T>: std::ops::Deref<Target = T> + Clone;
    fn new<T>(t: T) -> Self::Ptr<T>;
}
struct RcF;
struct ArcF;
impl PointerFamily for RcF {
    type Ptr<T> = std::rc::Rc<T>;
    fn new<T>(t: T) -> std::rc::Rc<T> { std::rc::Rc::new(t) }
}
impl PointerFamily for ArcF {
    type Ptr<T> = std::sync::Arc<T>;
    fn new<T>(t: T) -> std::sync::Arc<T> { std::sync::Arc::new(t) }
}
struct List<P: PointerFamily> {
    items: Vec<P::Ptr<String>>,
}
impl<P: PointerFamily> List<P> {
    fn push(&mut self, s: &str) { self.items.push(P::new(s.to_string())); }
    fn dup_last(&mut self) {
        let l = self.items.last().unwrap().clone();
        self.items.push(l);
    }
    fn total(&self) -> usize { self.items.iter().map(|p| p.len()).sum() }
}

fn main() {
    let mut v = vec![1, 2, 3, 4, 5];
    let mut w = WindowsMut { slice: &mut v, start: 0, size: 3 };
    while let Some(win) = w.next() {
        win[0] += win[2];
        println!("{:?}", win);
    }
    println!("{:?}", v);
    let l = Lines { buf: String::new(), data: vec!["a", "bb", "ccc"], idx: 0 };
    println!("count={}", count(l));
    let l = Lines { buf: String::new(), data: vec!["x", "yy"], idx: 0 };
    let mut acc = Vec::new();
    for_each(l, |s: &str| acc.push(s.len()));
    println!("{:?}", acc);
    let arr: &'static mut [i32] = Box::leak(Box::new([10, 20, 30]));
    let mut tot = 0;
    for_each(WindowsMut { slice: arr, start: 0, size: 2 }, |w: &mut [i32]| {
        w[1] -= 1;
        tot += w[0] + w[1];
    });
    println!("tot={}", tot);
    println!("{:?}", wrap_both::<OptionFamily>(1, "o"));
    println!("{:?}", wrap_both::<VecFamily>(2, "v"));
    let mut l1: List<RcF> = List { items: vec![] };
    l1.push("abc");
    l1.dup_last();
    println!("{} {}", l1.total(), std::rc::Rc::strong_count(&l1.items[0]));
    let mut l2: List<ArcF> = List { items: vec![] };
    l2.push("hello");
    l2.dup_last();
    l2.dup_last();
    println!("{} {}", l2.total(), std::sync::Arc::strong_count(&l2.items[0]));
}
