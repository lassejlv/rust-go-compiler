trait LendingIterator {
    type Item<'a> where Self: 'a;
    fn next<'a>(&'a mut self) -> Option<Self::Item<'a>>;
}

struct WindowsMut<'s, T> {
    data: &'s mut [T],
    pos: usize,
    size: usize,
}

impl<'s, T> LendingIterator for WindowsMut<'s, T> {
    type Item<'a> = &'a mut [T] where Self: 'a;
    fn next<'a>(&'a mut self) -> Option<&'a mut [T]> {
        if self.pos + self.size > self.data.len() { return None; }
        let w = &mut self.data[self.pos..self.pos + self.size];
        self.pos += 1;
        Some(w)
    }
}

struct LineBuf { text: String, buf: String, idx: usize }
impl LendingIterator for LineBuf {
    type Item<'a> = &'a str;
    fn next<'a>(&'a mut self) -> Option<&'a str> {
        let line = self.text.lines().nth(self.idx)?;
        self.idx += 1;
        self.buf.clear();
        self.buf.push_str(&line.to_uppercase());
        self.buf.push('|');
        Some(&self.buf)
    }
}

trait Family {
    type Member<T>;
    fn wrap<T>(t: T) -> Self::Member<T>;
}
struct OptFam;
struct VecFam;
impl Family for OptFam {
    type Member<T> = Option<T>;
    fn wrap<T>(t: T) -> Option<T> { Some(t) }
}
impl Family for VecFam {
    type Member<T> = Vec<T>;
    fn wrap<T>(t: T) -> Vec<T> { vec![t] }
}
fn wrap_both<F: Family>(a: i32, b: &str) -> (F::Member<i32>, F::Member<String>) {
    (F::wrap(a), F::wrap(b.to_string()))
}

trait Container {
    type Iter<'a>: Iterator<Item = &'a u32> where Self: 'a;
    fn iter<'a>(&'a self) -> Self::Iter<'a>;
    fn total(&self) -> u32 { self.iter().sum() }
}
struct Bag(Vec<u32>);
impl Container for Bag {
    type Iter<'a> = std::slice::Iter<'a, u32>;
    fn iter<'a>(&'a self) -> Self::Iter<'a> { self.0.iter() }
}
struct Evens(Vec<u32>);
impl Container for Evens {
    type Iter<'a> = std::iter::Filter<std::slice::Iter<'a, u32>, fn(&&u32) -> bool>;
    fn iter<'a>(&'a self) -> Self::Iter<'a> {
        fn even(x: &&u32) -> bool { **x % 2 == 0 }
        self.0.iter().filter(even as fn(&&u32) -> bool)
    }
}

fn main() {
    let mut arr = [1, 2, 3, 4, 5];
    let mut w = WindowsMut { data: &mut arr, pos: 0, size: 3 };
    while let Some(win) = w.next() {
        win[2] += win[0];
        println!("{:?}", win);
    }
    println!("{:?}", arr);
    let mut lb = LineBuf { text: "ab\ncd\nef".into(), buf: String::new(), idx: 0 };
    let mut out = vec![];
    while let Some(l) = lb.next() { out.push(l.to_string()); }
    println!("{:?}", out);
    println!("{:?}", wrap_both::<OptFam>(1, "x"));
    println!("{:?}", wrap_both::<VecFam>(2, "y"));
    let b = Bag(vec![1, 2, 3, 4]);
    let e = Evens(vec![1, 2, 3, 4, 6]);
    println!("{} {}", b.total(), e.total());
    println!("{:?}", e.iter().collect::<Vec<_>>());
}
