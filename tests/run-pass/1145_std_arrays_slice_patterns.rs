// Arrays and slice patterns: array::from_fn, map, each_ref/each_mut,
// TryFrom slice->array, as_chunks, concat/flatten of arrays, const generic
// helpers, slice pattern matching with rest/bindings, and array comparisons.
use std::convert::TryInto;

fn classify(v: &[i32]) -> String {
    match v {
        [] => "empty".to_string(),
        [x] => format!("one({})", x),
        [first, .., last] if first == last => format!("bookends({})", first),
        [first, second, rest @ ..] if rest.len() > 2 => format!("long: {} {} +{} more (sum {})", first, second, rest.len(), rest.iter().sum::<i32>()),
        [a, b] => format!("pair({}, {})", a, b),
        [head @ .., tail] => format!("head {:?} tail {}", head, tail),
    }
}

fn middle<T: Copy>(v: &[T]) -> Option<T> {
    match v {
        [_, inner @ .., _] if !inner.is_empty() => Some(inner[inner.len() / 2]),
        _ => None,
    }
}

fn parse_cmd(words: &[&str]) -> String {
    match words {
        ["go", dir] => format!("move {}", dir),
        ["take", item @ ..] if !item.is_empty() => format!("take {}", item.join(" ")),
        ["say", .., last] => format!("say ... {}", last),
        [cmd, args @ ..] => format!("unknown {} with {} args", cmd, args.len()),
        [] => "nothing".into(),
    }
}

fn sum_arr<const N: usize>(a: [u32; N]) -> u32 { a.iter().sum() }
fn first_last<T: Copy + Default, const N: usize>(a: &[T; N]) -> (T, T) {
    if N == 0 { (T::default(), T::default()) } else { (a[0], a[N - 1]) }
}
fn transpose<const R: usize, const C: usize>(m: [[i32; C]; R]) -> [[i32; R]; C] {
    std::array::from_fn(|c| std::array::from_fn(|r| m[r][c]))
}

fn main() {
    for v in [&[][..], &[7], &[1, 2], &[3, 9, 3], &[1, 2, 3, 4, 5, 6], &[1, 2, 3]] {
        println!("{:?} -> {}", v, classify(v));
    }
    println!("middle {:?} {:?} {:?}", middle(&[1, 2, 3, 4, 5]), middle(&['a', 'b']), middle::<u8>(&[]));
    for c in [&["go", "north"][..], &["take", "old", "lamp"], &["take"], &["say", "a", "b", "bye"], &["jump", "x"], &[]] {
        println!("{:?} => {}", c, parse_cmd(c));
    }
    let squares: [u64; 8] = std::array::from_fn(|i| (i as u64) * (i as u64));
    println!("from_fn {:?}", squares);
    let strs = [1, 2, 3].map(|x| format!("<{}>", x));
    println!("map {:?}", strs);
    let refs: [&String; 3] = strs.each_ref();
    println!("each_ref lens {:?}", refs.map(|s| s.len()));
    let mut nums = [1, 2, 3];
    for r in nums.each_mut() {
        *r *= 10;
    }
    println!("each_mut {:?}", nums);
    let slice: &[u8] = &[1, 2, 3, 4, 5];
    let arr3: Result<[u8; 3], _> = slice[..3].try_into();
    let arr_bad: Result<[u8; 3], _> = slice.try_into();
    println!("try_into {:?} {:?}", arr3, arr_bad.map_err(|e| e.to_string()));
    let aref: &[u8; 2] = slice[1..3].try_into().unwrap();
    println!("ref array {:?} u16 {}", aref, u16::from_le_bytes(*aref));
    let (chunks, rem) = slice.as_chunks::<2>();
    println!("as_chunks {:?} rem {:?}", chunks, rem);
    let (rrem, rchunks) = slice.as_rchunks::<2>();
    println!("as_rchunks {:?} {:?}", rrem, rchunks);
    let nested = [[1, 2], [3, 4], [5, 6]];
    println!("as_flattened {:?} concat {:?}", nested.as_flattened(), nested.concat());
    println!("transpose {:?}", transpose(nested));
    println!("sum_arr {} {} first_last {:?} {:?}", sum_arr([1, 2, 3]), sum_arr([]), first_last(&[4, 5, 6]), first_last::<char, 0>(&[]));
    let a = [3, 1, 2];
    let mut b = a;
    b.sort();
    println!("copy semantics {:?} {:?} eq {} lt {} cmp {:?}", a, b, a == b, b < a, [1, 2, 3].cmp(&[1, 2, 4]));
    println!("array eq slice {} vec {}", a == [3, 1, 2][..], vec![3, 1, 2] == a);
    let big = [0u8; 1000];
    println!("big len {} all zero {}", big.len(), big.iter().all(|&x| x == 0));
    let grid = [['.'; 4]; 3];
    let mut g2 = grid;
    g2[1][2] = '#';
    for row in &g2 {
        println!("{}", row.iter().collect::<String>());
    }
    let [x, y, z] = [10, 20, 30];
    let [first, .., last] = [1, 2, 3, 4, 5];
    let [ref r0, ref rest @ ..] = ["a", "b", "c"];
    println!("destructure {} {} {} {} {} {} {:?}", x, y, z, first, last, r0, rest);
    if let [a, b, ..] = &mut nums[..] {
        std::mem::swap(a, b);
    }
    println!("swapped via pattern {:?}", nums);
    let mut stack = vec![1, 2, 3, 4];
    while let [.., top] = stack[..] {
        if top < 3 {
            break;
        }
        stack.pop();
    }
    println!("while let slice {:?}", stack);
    let text = "a=1;b=2;c";
    for part in text.split(';') {
        match part.split('=').collect::<Vec<_>>().as_slice() {
            [k, v] => println!("kv {} -> {}", k, v),
            [k] => println!("flag {}", k),
            _ => println!("weird"),
        }
    }
    let pairs = [(1, 'a'), (2, 'b')];
    let [(n1, c1), (n2, c2)] = pairs;
    println!("{} {} {} {}", n1, c1, n2, c2);
    let matrix: [[f32; 2]; 2] = [[1.0, 2.0], [3.0, 4.0]];
    let det = matrix[0][0] * matrix[1][1] - matrix[0][1] * matrix[1][0];
    println!("det {} iter flat {:?}", det, matrix.iter().flatten().map(|v| v * 2.0).collect::<Vec<_>>());
    let bytes = *b"hey";
    println!("byte string array {:?} {:?}", bytes, std::str::from_utf8(&bytes));
    let boxed: Box<[i32; 4]> = Box::new([4, 3, 2, 1]);
    let v: Vec<i32> = boxed.into_iter().rev().collect();
    println!("boxed array into_iter {:?}", v);
    let arr_of_vec: [Vec<u8>; 3] = Default::default();
    println!("default {:?} {:?}", arr_of_vec, <[bool; 2]>::default());
    let opt_arr: [Option<String>; 2] = [None, Some("x".into())];
    println!("{:?}", opt_arr.iter().flatten().collect::<Vec<_>>());
}
