use std::mem::size_of;
use std::fmt::Debug;

struct Foo<const N: usize = 3>([u8; N]);
impl<const N: usize> Foo<N> { fn n(&self) -> usize { N } }

fn check<const N: usize>() -> usize {
    const { assert!(N > 0) };
    N * 2
}

fn nones<T>() -> [Option<T>; 4] { [const { None }; 4] }

trait Describe { fn describe(&self) -> String; }
impl<T: Debug, const N: usize> Describe for [T; N] {
    fn describe(&self) -> String { format!("array of {} : {:?}", N, self) }
}
impl<T: Debug> Describe for [T] {
    fn describe(&self) -> String { format!("slice of {}", self.len()) }
}

struct Sz<T>(T);
impl<T> Sz<T> {
    const SIZE: usize = size_of::<T>();
    const IS_ZST: bool = Self::SIZE == 0;
}

trait HasLen { const LEN: usize; }
struct Three;
impl HasLen for Three { const LEN: usize = 3; }

fn sum_n<const N: usize>(xs: [i32; N]) -> i32 where [i32; N]: Debug { xs.iter().sum() }

fn main() {
    let f: Foo = Foo([1, 2, 3]);
    let g = Foo::<5>([0; 5]);
    println!("{} {}", f.n(), g.n());
    println!("{} {}", check::<4>(), check::<1>());
    let ns: [Option<String>; 4] = nones();
    println!("{:?}", ns);
    println!("{}", [1, 2].describe());
    println!("{}", [1, 2][..].describe());
    println!("{}", ["x"; 3].describe());
    println!("{} {} {} {}", Sz::<u64>::SIZE, Sz::<()>::IS_ZST, Sz::<[u16; 3]>::SIZE, Sz::<(u8, u32)>::SIZE);
    let arr = [7u8; <Three as HasLen>::LEN];
    println!("{:?}", arr);
    println!("{}", sum_n([1, 2, 3, 4]));
    println!("{} {} {} {}", size_of::<Option<Box<i32>>>(), size_of::<Option<u32>>(), size_of::<Option<bool>>(), size_of::<Option<char>>());
    println!("{} {} {}", size_of::<Option<Option<bool>>>(), size_of::<Option<&str>>(), size_of::<Result<u8, ()>>());
    println!("{} {}", size_of::<Option<Vec<u8>>>(), size_of::<Option<String>>());
    println!("{}", size_of::<Option<std::num::NonZeroU32>>());
}
