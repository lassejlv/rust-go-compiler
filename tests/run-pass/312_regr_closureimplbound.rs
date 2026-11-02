pub trait Ext<P>: Sized { fn take_until(self, predicate: P) -> TU<Self, P>; }
impl<I: Iterator, P: FnMut(&I::Item) -> bool> Ext<P> for I {
    fn take_until(self, predicate: P) -> TU<Self, P> { TU { iter: self, flag: false, predicate } }
}
pub struct TU<I, P> { iter: I, flag: bool, predicate: P }
impl<I: Iterator, P: FnMut(&I::Item) -> bool> Iterator for TU<I, P> {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> {
        if self.flag { return None; }
        let x = self.iter.next()?;
        if (self.predicate)(&x) { self.flag = true; }
        Some(x)
    }
}
fn main() { println!("{:?}", (1..10).take_until(|x| *x == 4).collect::<Vec<_>>()); }
