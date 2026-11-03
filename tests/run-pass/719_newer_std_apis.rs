// Newer stable std APIs (1.77 - 1.9x): chunk_by, is_sorted, extract_if,
// pop_if, get_disjoint_mut, as_chunks, as_flattened, repeat_n, is_none_or,
// take_if, isqrt, midpoint, div_ceil, ilog, Cell::update, LazyCell.
use std::cell::{Cell, LazyCell};
use std::collections::HashMap;

fn main() {
    // slice::chunk_by
    let v = [1, 1, 2, 3, 3, 3, 2, 2, 5];
    let groups: Vec<&[i32]> = v.chunk_by(|a, b| a == b).collect();
    println!("chunk_by eq: {:?}", groups);
    let runs: Vec<Vec<i32>> = [1, 2, 3, 2, 3, 4, 1].chunk_by(|a, b| a < b).map(|s| s.to_vec()).collect();
    println!("ascending runs: {:?}", runs);
    let mut m = [5, 5, 1, 1, 1, 9];
    for g in m.chunk_by_mut(|a, b| a == b) {
        let n = g.len() as i32;
        g.iter_mut().for_each(|x| *x *= n);
    }
    println!("chunk_by_mut: {:?}", m);

    // is_sorted family
    println!("is_sorted: {} {} {}", [1, 2, 2, 9].is_sorted(), [3, 1].is_sorted(), ["b", "a"].is_sorted_by_key(|s| std::cmp::Reverse(*s)));
    println!("iter is_sorted_by: {}", (1..10).map(|x| x * x).is_sorted_by(|a, b| a < b));

    // Vec::extract_if / pop_if / retain_mut
    let mut nums: Vec<i32> = (1..=12).collect();
    let evens: Vec<i32> = nums.extract_if(.., |x| *x % 2 == 0).collect();
    println!("extract_if evens={:?} rest={:?}", evens, nums);
    let popped = nums.pop_if(|x| *x > 10);
    let not_popped = nums.pop_if(|x| *x > 100);
    println!("pop_if {:?} {:?} -> {:?}", popped, not_popped, nums);
    nums.retain_mut(|x| { *x *= 10; *x != 50 });
    println!("retain_mut: {:?}", nums);

    // get_disjoint_mut
    let mut arr = [10, 20, 30, 40];
    if let Ok([a, b]) = arr.get_disjoint_mut([0, 3]) {
        std::mem::swap(a, b);
    }
    let overlap = arr.get_disjoint_mut([1, 1]).is_err();
    println!("disjoint swap: {:?} overlap err: {:?}", arr, overlap);
    let mut hm: HashMap<&str, i32> = HashMap::from([("a", 1), ("b", 2), ("c", 3)]);
    let [x, y] = hm.get_disjoint_mut(["a", "c"]);
    if let (Some(x), Some(y)) = (x, y) { *x += 100; *y += 200; }
    let mut items: Vec<_> = hm.into_iter().collect();
    items.sort();
    println!("hashmap disjoint: {:?}", items);

    // as_chunks / as_flattened / split_first_chunk
    let bytes = [1u8, 2, 3, 4, 5, 6, 7];
    let (chunks, rem) = bytes.as_chunks::<3>();
    println!("as_chunks: {:?} rem {:?}", chunks, rem);
    let (rchunks_rem, rchunks) = bytes.as_rchunks::<2>();
    println!("as_rchunks: rem {:?} {:?}", rchunks_rem, rchunks);
    let grid = [[1, 2], [3, 4], [5, 6]];
    println!("as_flattened: {:?}", grid.as_flattened());
    if let Some((head, tail)) = bytes.split_first_chunk::<2>() {
        println!("split_first_chunk: {:?} {:?}", head, tail);
    }
    println!("first_chunk: {:?} last_chunk: {:?}", bytes.first_chunk::<4>(), bytes.last_chunk::<9>());

    // iterators
    let rn: Vec<String> = std::iter::repeat_n("ab".to_string(), 3).collect();
    println!("repeat_n: {:?}", rn);
    let mw: Vec<i32> = ["1", "2", "x", "4"].iter().map_while(|s| s.parse().ok()).collect();
    println!("map_while: {:?}", mw);
    let z: Vec<(char, u8)> = std::iter::zip("abc".chars(), 1u8..).collect();
    println!("iter::zip: {:?}", z);

    // Option helpers
    let a: Option<i32> = None;
    let b = Some(5);
    println!("is_none_or: {} {} {}", a.is_none_or(|x| x > 10), b.is_none_or(|x| x > 10), b.is_some_and(|x| x == 5));
    let mut t = Some(42);
    let took = t.take_if(|v| *v % 2 == 0);
    println!("take_if: {:?} left {:?}", took, t);
    let mut s = Some(3);
    println!("take_if no: {:?} left {:?}", s.take_if(|v| *v > 3), s);

    // integer math
    println!("isqrt: {} {} {}", 99u32.isqrt(), 100u64.isqrt(), i32::MAX.isqrt());
    println!("midpoint: {} {} {}", 3u8.midpoint(250), 0u32.midpoint(u32::MAX), 1.5f64.midpoint(4.0));
    println!("div_ceil: {} {} ilog: {} {} {}", 7u32.div_ceil(2), 8u64.div_ceil(4), 1000u32.ilog10(), 1024u32.ilog2(), 80u32.ilog(3));
    println!("checked_ilog10(0)={:?} unsigned_abs={} abs_diff={}", 0u8.checked_ilog10(), i8::MIN.unsigned_abs(), 3u8.abs_diff(250));
    println!("next_multiple_of: {} {}", 13u32.next_multiple_of(5), 15u32.next_multiple_of(5));
    println!("cast_signed/unsigned: {} {}", 200u8.cast_signed(), (-1i16).cast_unsigned());

    // Cell::update and LazyCell
    let c = Cell::new(10);
    c.update(|x| x * 3);
    println!("Cell::update -> {}", c.get());
    let calls = Cell::new(0);
    let lazy = LazyCell::new(|| { calls.set(calls.get() + 1); (1..=5).product::<u64>() });
    println!("lazy before: calls={}", calls.get());
    println!("lazy value {} {} calls={}", *lazy, *lazy + 1, calls.get());

    // inline const and exclusive range patterns
    let table = const { [1u32 << 3, 1 << 5, 1 << 7] };
    for n in [0u32, 5, 10, 99, 100, 1000] {
        let cls = match n {
            0 => "zero",
            1..10 => "single",
            10..100 => "double",
            100.. => "big",
        };
        print!("{}:{} ", n, cls);
    }
    println!("{:?}", table);
    #[derive(Debug)]
    #[allow(dead_code)]
    struct S { a: u8, b: u32, c: u16 }
    println!("offset_of b={} c={}", std::mem::offset_of!(S, b), std::mem::offset_of!(S, c));
}
