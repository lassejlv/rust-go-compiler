const fn square(x: u32) -> u32 { x * x }

const fn fib_table<const N: usize>() -> [u64; N] {
    let mut t = [0u64; N];
    if N > 1 { t[1] = 1; }
    let mut i = 2;
    while i < N {
        t[i] = t[i - 1] + t[i - 2];
        i += 1;
    }
    t
}

const FIBS: [u64; 12] = fib_table::<12>();

struct Wrapper<const N: usize>([u8; N]);
impl<const N: usize> Wrapper<N> {
    const LEN: usize = N;
    const DOUBLE: usize = N * 2;
    fn total(&self) -> u32 { self.0.iter().map(|&b| b as u32).sum() }
}

fn gen<T: Default + Copy, const N: usize>() -> [T; N] { [T::default(); N] }

fn main() {
    let a = [1, 2, 3, 4];
    let sq = a.map(|x| x * x);
    println!("{:?}", sq);
    let strs = ["a", "bb", "ccc"].map(str::len);
    println!("{:?}", strs);
    let owned = [1, 2, 3].map(|x| format!("<{}>", x));
    println!("{:?}", owned);
    let arr2: [[u8; 2]; 3] = [1, 2, 3].map(|x| [x, x * 10]);
    println!("{:?}", arr2);
    let made: [usize; 5] = std::array::from_fn(|i| i * i + 1);
    println!("{:?}", made);
    let idx: [String; 3] = core::array::from_fn(|i| "z".repeat(i));
    println!("{:?}", idx);
    let x = const { square(12) + 1 };
    println!("const block {}", x);
    let v: [Vec<i32>; 3] = [const { Vec::new() }; 3];
    println!("{:?}", v);
    let arr_of_opts: [Option<String>; 4] = [const { None }; 4];
    println!("{}", arr_of_opts.iter().all(Option::is_none));
    println!("{:?}", FIBS);
    const SQ: [u32; 5] = { let mut a = [0; 5]; let mut i = 0; while i < 5 { a[i] = square(i as u32); i += 1; } a };
    println!("{:?}", SQ);
    let w = Wrapper([1, 2, 3, 4, 5]);
    println!("{} {} {}", Wrapper::<5>::LEN, Wrapper::<5>::DOUBLE, w.total());
    let g: [i64; 3] = gen();
    let gb = gen::<bool, 2>();
    println!("{:?} {:?}", g, gb);
    let [first, .., last] = a;
    let [_, mid @ .., _] = a;
    println!("{} {} {:?}", first, last, mid);
    let zipped: [(i32, char); 3] = {
        let cs = ['x', 'y', 'z'];
        let mut i = 0;
        [0; 3].map(|_| { let r = (a[i], cs[i]); i += 1; r })
    };
    println!("{:?}", zipped);
    let total: i32 = a.into_iter().sum();
    println!("{}", total);
    let ok: Result<[u8; 3], _> = <[u8; 3]>::try_from(&[9u8, 8, 7][..]);
    let bad: Result<[u8; 3], _> = <[u8; 3]>::try_from(&[1u8][..]);
    println!("{:?} {}", ok, bad.is_err());
    let r: &[i32; 2] = a[1..3].try_into().unwrap();
    println!("{:?}", r);
    println!("{}", [1u8; 3] == [1, 1, 1]);
    let nested = [[0u8; 3]; 2].map(|row| row.map(|c| c + 1));
    println!("{:?}", nested);
    println!("{:?}", a.each_ref().map(|r| *r * 100));
    let mut m = [1, 2, 3];
    for r in m.each_mut() { *r += 10; }
    println!("{:?}", m);
    const C: usize = const { 3 + 4 };
    println!("{}", [0; C].len());
    println!("{:?}", a.as_slice().iter().rev().collect::<Vec<_>>());
}
