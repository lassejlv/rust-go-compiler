use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};

fn main() {
    let v: Vec<i32> = (1..=5).collect();
    println!("vec: {:?}", v);
    let s: String = vec!['h', 'i', '!'].into_iter().collect();
    println!("string from chars: {}", s);
    let s2: String = vec!["ab", "cd"].into_iter().collect();
    println!("string from strs: {}", s2);
    let s3: String = (0..5).map(|i| i.to_string()).collect();
    println!("string from Strings: {}", s3);
    let m: HashMap<&str, i32> = vec![("one", 1), ("two", 2), ("three", 3)].into_iter().collect();
    let mut keys: Vec<&&str> = m.keys().collect();
    keys.sort();
    println!("hashmap keys sorted: {:?}", keys);
    println!("hashmap get two: {:?}", m.get("two"));
    let hs: HashSet<i32> = [1, 2, 2, 3, 3, 3].iter().cloned().collect();
    println!("hashset len: {}", hs.len());
    let bm: BTreeMap<char, usize> = "hello".chars().rev().enumerate().map(|(i, c)| (c, i)).collect();
    println!("btreemap: {:?}", bm);
    let bs: BTreeSet<char> = "mississippi".chars().collect();
    println!("btreeset: {:?}", bs);
    let dq: VecDeque<i32> = (1..4).collect();
    println!("vecdeque: {:?}", dq);
    let res: Result<Vec<i32>, String> = vec![Ok(1), Ok(2)].into_iter().collect();
    println!("result ok: {:?}", res);
    let res2: Result<Vec<i32>, String> = vec![Ok(1), Err("bad".to_string()), Ok(3)].into_iter().collect();
    println!("result err: {:?}", res2);
    let opt: Option<Vec<char>> = "123".chars().map(|c| if c.is_ascii_digit() { Some(c) } else { None }).collect();
    println!("option: {:?}", opt);
    let sum_opt: Option<u32> = "123".chars().map(|c| c.to_digit(10)).sum();
    println!("option sum: {:?}", sum_opt);
    let boxed: Box<[i32]> = (1..4).collect();
    println!("boxed slice: {:?}", boxed);
    let mut inspected = Vec::new();
    let total: i32 = (1..=4).inspect(|x| inspected.push(*x)).map(|x| x * 10).sum();
    println!("inspect: total={} seen={:?}", total, inspected);
    let index: HashMap<char, Vec<&str>> = ["apple", "avocado", "beet", "carrot", "cucumber"]
        .iter()
        .fold(HashMap::new(), |mut acc, w| {
            acc.entry(w.chars().next().unwrap()).or_insert_with(Vec::new).push(*w);
            acc
        });
    let mut entries: Vec<(&char, &Vec<&str>)> = index.iter().collect();
    entries.sort();
    println!("grouped: {:?}", entries);
    let lens: Vec<usize> = vec!["a", "bbb", "cc"].iter().map(|s| s.len()).collect::<Vec<_>>();
    println!("turbofish collect: {:?}", lens);
    let mut extended = vec![0];
    extended.extend((1..4).map(|x| x * x));
    println!("extend from iter: {:?}", extended);
    let chunks: Vec<Vec<i32>> = (1..=7).collect::<Vec<_>>().chunks(3).map(|c| c.to_vec()).collect();
    println!("chunked: {:?}", chunks);
}
