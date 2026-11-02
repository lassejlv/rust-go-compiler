trait Ext: Iterator {
    fn pairs<J>(self, other: J) -> P<Self, J::IntoIter> where Self: Sized, Self::Item: Clone, J: IntoIterator, J::IntoIter: Clone { P(self, other.into_iter()) }
}
impl<I: Iterator + ?Sized> Ext for I {}
struct P<A, B>(A, B);
impl<A: Iterator, B: Clone + Iterator> Iterator for P<A, B> where A::Item: Clone {
    type Item = (A::Item, B::Item);
    fn next(&mut self) -> Option<Self::Item> { Some((self.0.next()?, self.1.next()?)) }
}
fn main() {
    let v: Vec<_> = Ext::pairs(IntoIterator::into_iter(0..3), IntoIterator::into_iter(5..9)).collect();
    println!("{:?}", v);
}
