use std::fmt::Debug;
use std::ops::Add;

trait Lend { type Item<'a> where Self: 'a; fn lend(&mut self) -> Option<Self::Item<'_>>; }
struct Buf { data: Vec<u32>, i: usize }
impl Lend for Buf {
    type Item<'a> = &'a mut u32;
    fn lend(&mut self) -> Option<&mut u32> { let i = self.i; self.i += 1; self.data.get_mut(i) }
}
fn bump_all<L>(l: &mut L) -> usize where L: Lend, for<'a> <L as Lend>::Item<'a>: std::ops::DerefMut<Target = u32> {
    let mut n = 0;
    while let Some(mut x) = l.lend() { *x += 100; n += 1; }
    n
}

trait Store { type Out: Debug + Default; const N: usize; fn get(&self) -> Self::Out; }
struct A1; struct A2;
impl Store for A1 { type Out = Vec<u8>; const N: usize = 2; fn get(&self) -> Vec<u8> { vec![1] } }
impl Store for A2 { type Out = (i8, char); const N: usize = 5; fn get(&self) -> (i8, char) { (-1, 'q') } }
type OutOf<T> = <T as Store>::Out;
fn defaults<T: Store>() -> OutOf<T> { OutOf::<T>::default() }

trait Num2: Add<Output = Self> + Copy + Debug {}
impl<T: Add<Output = T> + Copy + Debug> Num2 for T {}
fn add3<T: Num2>(a: T, b: T, c: T) -> T { a + b + c }

trait HasItem { type It; fn item(&self) -> Self::It; }
impl HasItem for String { type It = usize; fn item(&self) -> usize { self.len() } }
impl HasItem for bool { type It = &'static str; fn item(&self) -> &'static str { if *self { "yes" } else { "no" } } }
fn show_items<T, U>(xs: &[T]) -> Vec<U> where T: HasItem<It = U>, U: Debug { xs.iter().map(|x| x.item()).collect() }

static INC: fn(i32) -> i32 = |x| x + 1;
const DBL: fn(i32) -> i32 = |x| x * 2;
const fn size<T>() -> usize { std::mem::size_of::<T>() }
const S8: usize = size::<u64>() + size::<(u8, u16)>();

struct Grid;
impl Grid { const N: usize = 4; fn arr() -> [u8; Self::N] { [7; Self::N] } }
struct Cn<const N: usize>;
impl<const N: usize> Cn<N> { const DOUBLE: usize = N * 2; }

fn main() {
    let mut b = Buf { data: vec![1, 2, 3], i: 0 };
    println!("{} {:?}", bump_all(&mut b), b.data);
    println!("{:?} {:?}", defaults::<A1>(), defaults::<A2>());
    let o: OutOf<A2> = A2.get();
    println!("{:?} {} {}", o, <A1 as Store>::N + A2::N, A1.get().len());
    println!("{} {:?}", add3(1, 2, 3), add3(0.5, 0.25, 0.125));
    println!("{:?} {:?}", show_items(&["ab".to_string(), "cde".to_string()]), show_items(&[true, false]));
    println!("{} {} {}", INC(1), DBL(4), S8);
    let arr = Grid::arr();
    let arr2 = [0u16; Cn::<3>::DOUBLE];
    println!("{:?} {:?} {}", arr, arr2, Cn::<10>::DOUBLE);
    let fs: [fn(i32) -> i32; 2] = [INC, DBL];
    println!("{:?}", fs.iter().map(|f| f(10)).collect::<Vec<_>>());
}
