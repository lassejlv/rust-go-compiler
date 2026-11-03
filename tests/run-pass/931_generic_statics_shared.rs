use std::sync::atomic::{AtomicUsize, Ordering};
use std::cell::Cell;

fn count_calls<T>(_x: T) -> usize {
    static CALLS: AtomicUsize = AtomicUsize::new(0);
    CALLS.fetch_add(1, Ordering::SeqCst) + 1
}

fn type_name_len<T>() -> usize { std::any::type_name::<T>().len() }

struct Counter<T> { _p: std::marker::PhantomData<T> }
impl<T> Counter<T> {
    fn bump() -> usize {
        static N: AtomicUsize = AtomicUsize::new(100);
        N.fetch_add(1, Ordering::SeqCst)
    }
}

thread_local! {
    static DEPTH: Cell<u32> = Cell::new(0);
}

fn nested<T: std::fmt::Debug>(x: T, n: u32) -> String {
    DEPTH.with(|d| d.set(d.get() + 1));
    let r = if n == 0 { format!("{:?}@{}", x, DEPTH.with(|d| d.get())) } else { nested(x, n - 1) };
    r
}

static GLOBAL: AtomicUsize = AtomicUsize::new(0);
trait Tick { fn tick(&self) -> usize { GLOBAL.fetch_add(1, Ordering::Relaxed) } }
impl Tick for u8 {}
impl Tick for String {}
impl<T: Tick> Tick for Vec<T> { fn tick(&self) -> usize { self.iter().map(|t| t.tick()).sum() } }

const fn square(x: usize) -> usize { x * x }
struct Grid<const N: usize>;
impl<const N: usize> Grid<N> {
    const CELLS: usize = square(N);
    fn cells(&self) -> [u8; N] { [N as u8; N] }
}

fn main() {
    println!("{}", count_calls(1i32));
    println!("{}", count_calls("a"));
    println!("{}", count_calls(2.5f64));
    println!("{}", count_calls(3i32));
    println!("{} {} {}", Counter::<u8>::bump(), Counter::<String>::bump(), Counter::<u8>::bump());
    println!("{}", type_name_len::<u8>() < type_name_len::<Vec<u8>>());
    println!("{}", nested(5u8, 3));
    println!("{}", nested("s", 1));
    println!("{}", 1u8.tick());
    println!("{}", String::new().tick());
    println!("{}", vec![1u8, 2, 3].tick());
    println!("{}", GLOBAL.load(Ordering::Relaxed));
    println!("{} {:?}", Grid::<4>::CELLS, Grid::<3>.cells());
}
