pub trait Coll: Sized { type Item; fn first(&self) -> Self::Item; }
pub trait Homo: Coll {}
impl<T: Coll> Homo for T {}
impl Coll for (i32, i32) { type Item = i32; fn first(&self) -> i32 { self.0 } }
pub struct W<I, T> where I: Iterator<Item = T::Item>, T: Homo { it: I, t: T }
fn get<T: Homo>(t: &T) -> T::Item { t.first() }
fn main() {
    let w = W { it: vec![1, 2].into_iter(), t: (5, 6) };
    println!("{} {}", get(&w.t), w.it.count());
}
