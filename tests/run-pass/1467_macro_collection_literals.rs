// collection literal macros: hashmap!, btreeset!, vec of vec!, array repeat
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

macro_rules! hashmap {
    (@single $($x:tt)*) => (());
    (@count $($rest:expr),*) => (<[()]>::len(&[$(hashmap!(@single $rest)),*]));
    ($($key:expr => $value:expr,)+) => { hashmap!($($key => $value),+) };
    ($($key:expr => $value:expr),*) => {{
        let _cap = hashmap!(@count $($key),*);
        let mut _map = HashMap::with_capacity(_cap);
        $( let _ = _map.insert($key, $value); )*
        _map
    }};
}

macro_rules! collection {
    ($($k:expr => $v:expr),* $(,)?) => {{
        core::convert::From::from([$(($k, $v),)*])
    }};
    ($($v:expr),* $(,)?) => {{
        core::convert::From::from([$($v,)*])
    }};
}

macro_rules! deque {
    ($elem:expr; $n:expr) => { VecDeque::from(vec![$elem; $n]) };
    ($($x:expr),*) => { VecDeque::from(vec![$($x),*]) };
}

fn main() {
    let m = hashmap!{ "a" => 1, "b" => 2, "c" => 3, };
    let mut keys: Vec<_> = m.iter().collect();
    keys.sort();
    println!("{:?}", keys);
    let empty: HashMap<i32, i32> = hashmap!();
    println!("{}", empty.len());
    let bm: BTreeMap<&str, i32> = collection! { "x" => 1, "y" => 2 };
    let bs: BTreeSet<char> = collection! { 'q', 'a', 'q' };
    let v: Vec<u8> = collection![3, 1, 2];
    println!("{:?} {:?} {:?}", bm, bs, v);
    let d = deque![0u8; 3];
    let d2 = deque![1, 2, 3];
    println!("{:?} {:?}", d, d2);
    let grid = vec![vec!['.'; 3]; 2];
    println!("{:?}", grid);
    let arr = [[0u8; 2]; 3];
    println!("{:?}", arr);
    let nested = vec![vec![], vec![1], vec![1, 2]];
    println!("{:?}", nested.iter().map(Vec::len).collect::<Vec<_>>());
    let strings = vec![String::from("s"); 2];
    println!("{:?}", strings);
    let v2: Vec<i32> = vec![];
    println!("{:?} {}", v2, vec![1; 0].len());
}
