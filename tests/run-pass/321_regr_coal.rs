trait CountItem<T> { type CItem; fn new(t: T) -> Self::CItem; }
trait Pred<A, B> { fn ok(&mut self, a: &A, b: &B) -> bool; }
struct NoCount;
impl<T> CountItem<T> for NoCount { type CItem = T; fn new(t: T) -> T { t } }
struct F;
impl Pred<i32, i32> for F { fn ok(&mut self, a: &i32, b: &i32) -> bool { a == b } }
struct CB<I, P, C> { it: I, p: P, c: std::marker::PhantomData<C> }
impl<I, P, C> CB<I, P, C> where I: Iterator, P: Pred<I::Item, C::CItem>, C: CountItem<I::Item> {
    fn run(mut self) -> usize { let mut n = 0; while let Some(x) = self.it.next() { let y = C::new(x); let _ = &y; n += 1; } n }
}
fn main() { let c: CB<_, F, NoCount> = CB { it: vec![1, 2, 3].into_iter(), p: F, c: std::marker::PhantomData }; println!("{}", c.run()); }
