use std::fmt::Debug;

trait Functor {
    type Item;
    type Mapped<U>: Functor<Item = U>;
    fn fmap<U, F: FnMut(Self::Item) -> U>(self, f: F) -> Self::Mapped<U>;
}
impl<T> Functor for Option<T> {
    type Item = T;
    type Mapped<U> = Option<U>;
    fn fmap<U, F: FnMut(T) -> U>(self, f: F) -> Option<U> { self.map(f) }
}
impl<T> Functor for Vec<T> {
    type Item = T;
    type Mapped<U> = Vec<U>;
    fn fmap<U, F: FnMut(T) -> U>(self, f: F) -> Vec<U> { self.into_iter().map(f).collect() }
}
fn stringify<F: Functor>(f: F) -> F::Mapped<String> where F::Item: Debug {
    f.fmap(|x| format!("{:?}", x))
}
fn twice_mapped<F>(f: F) -> <F::Mapped<i64> as Functor>::Mapped<String>
where F: Functor<Item = i32> {
    f.fmap(|x| x as i64 * 2).fmap(|y| format!("<{}>", y))
}

trait Container {
    type Item;
    type Iter<'a>: Iterator<Item = &'a Self::Item> where Self: 'a;
    fn iter<'a>(&'a self) -> Self::Iter<'a>;
}
impl<T> Container for Vec<T> {
    type Item = T;
    type Iter<'a> = std::slice::Iter<'a, T> where T: 'a;
    fn iter<'a>(&'a self) -> std::slice::Iter<'a, T> { self.as_slice().iter() }
}
fn last_two<C: Container>(c: &C) -> Vec<&C::Item> where for<'a> C::Iter<'a>: DoubleEndedIterator {
    c.iter().rev().take(2).collect()
}

trait Lending { type Item<'a> where Self: 'a; fn next(&mut self) -> Option<Self::Item<'_>>; }
struct Adapt<I>(I);
impl<I: Iterator> Lending for Adapt<I> {
    type Item<'a> = I::Item where Self: 'a;
    fn next(&mut self) -> Option<I::Item> { self.0.next() }
}
struct ChunksMut<'s, T> { data: &'s mut [T], pos: usize, n: usize }
impl<'s, T> Lending for ChunksMut<'s, T> {
    type Item<'a> = &'a mut [T] where Self: 'a;
    fn next(&mut self) -> Option<&mut [T]> {
        if self.pos >= self.data.len() { return None; }
        let end = (self.pos + self.n).min(self.data.len());
        let r = &mut self.data[self.pos..end];
        self.pos = end;
        Some(r)
    }
}

fn show_rec(depth: u32, x: &dyn Debug) -> String {
    if depth == 0 { format!("{:?}", x) } else { show_rec(depth - 1, &(depth, x)) }
}

fn with_sink<F>(f: F) -> Vec<i32> where F: Fn(&mut dyn FnMut(i32)) {
    let mut v = vec![];
    f(&mut |x| v.push(x));
    v
}

fn main() {
    println!("{:?}", stringify(Some(3)));
    println!("{:?}", stringify(vec!['a', 'b']));
    println!("{:?}", twice_mapped(vec![1, 2, 3]));
    println!("{:?}", twice_mapped(None::<i32>));
    println!("{:?}", last_two(&vec![1, 2, 3, 4]));
    let mut a = Adapt("xyz".chars());
    while let Some(c) = a.next() { print!("{} ", c); }
    println!();
    let mut d = [1, 2, 3, 4, 5, 6, 7];
    let mut cm = ChunksMut { data: &mut d, pos: 0, n: 3 };
    while let Some(ch) = cm.next() { ch.reverse(); }
    println!("{:?}", d);
    println!("{}", show_rec(3, &"base"));
    println!("{:?}", with_sink(|emit| { for i in 0..4 { emit(i * i); } }));
}
