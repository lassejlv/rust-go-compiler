// collect into many targets: HashMap, BTreeMap, BTreeSet, String, Result<Vec>, Option<Vec>, Box<[T]>, VecDeque
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque, LinkedList, BinaryHeap};
use std::num::ParseIntError;

fn parse_all(v: &[&str]) -> Result<Vec<i32>, ParseIntError> {
    v.iter().map(|s| s.parse::<i32>()).collect()
}

fn main() {
    let words = ["apple", "bob", "cat", "apple", "dog", "cat", "apple"];
    let hm: HashMap<&str, usize> = words.iter().map(|w| (*w, w.len())).collect();
    let mut keys: Vec<_> = hm.iter().collect();
    keys.sort();
    println!("{:?}", keys);
    let bm: BTreeMap<usize, Vec<&str>> = words.iter().fold(BTreeMap::new(), |mut m, w| {
        m.entry(w.len()).or_insert_with(Vec::new).push(*w);
        m
    });
    println!("{:?}", bm);
    let bs: BTreeSet<char> = words.iter().flat_map(|w| w.chars()).collect();
    println!("{:?}", bs);
    let hs: HashSet<&str> = words.iter().cloned().collect();
    println!("{}", hs.len());
    let s: String = words.iter().map(|w| w.chars().next().unwrap()).collect();
    println!("{}", s);
    let s2: String = vec!["ab", "cd", "ef"].into_iter().collect();
    let s3: String = vec![String::from("x"), String::from("y")].into_iter().collect();
    println!("{} {}", s2, s3);
    println!("{:?}", parse_all(&["1", "2", "3"]));
    println!("{:?}", parse_all(&["1", "zz", "3", "q"]));
    let ov: Option<Vec<u32>> = ["1", "2"].iter().map(|s| s.parse().ok()).collect();
    let ov2: Option<Vec<u32>> = ["1", "-2"].iter().map(|s| s.parse().ok()).collect();
    println!("{:?} {:?}", ov, ov2);
    let bx: Box<[i32]> = (1..5).collect();
    println!("{:?} {}", bx, bx.len());
    let vd: VecDeque<i32> = (0..5).rev().collect();
    println!("{:?}", vd);
    let ll: LinkedList<&str> = ["x", "y", "z"].into_iter().collect();
    println!("{:?}", ll);
    let bh: BinaryHeap<i32> = vec![3, 1, 4, 1, 5].into_iter().collect();
    println!("{:?}", bh.into_sorted_vec());
    let pairs: Vec<(i32, char)> = vec![(1, 'a'), (2, 'b'), (3, 'c')];
    let (nums, chars): (Vec<i32>, String) = pairs.iter().cloned().unzip();
    println!("{:?} {}", nums, chars);
    let inverted: BTreeMap<char, i32> = pairs.iter().map(|&(n, c)| (c, n * n)).collect();
    println!("{:?}", inverted);
    let r: Result<String, String> = vec![Ok('h'), Ok('i'), Err("bad".to_string()), Ok('!')].into_iter().collect();
    println!("{:?}", r);
    let r2: Result<BTreeSet<i32>, &str> = vec![Ok(3), Ok(1), Ok(3)].into_iter().collect();
    println!("{:?}", r2);
    let total: Option<i32> = [Some(1), Some(2), Some(3)].into_iter().sum();
    let total2: Option<i32> = [Some(1), None, Some(3)].into_iter().sum();
    let prod: Result<i64, String> = vec![Ok(2i64), Ok(5)].into_iter().product();
    println!("{:?} {:?} {:?}", total, total2, prod);
    let mut ext = vec![0];
    ext.extend((1..3).map(|x| x * 10));
    ext.extend(&[7, 8]);
    println!("{:?}", ext);
    let mut hm2: HashMap<String, i32> = HashMap::new();
    hm2.extend(vec![("a".to_string(), 1), ("b".to_string(), 2)]);
    hm2.extend([("a".to_string(), 10)]);
    let mut v2: Vec<_> = hm2.into_iter().collect();
    v2.sort();
    println!("{:?}", v2);
    let from_arr = HashMap::from([(1, "one"), (2, "two")]);
    let bt = BTreeMap::from_iter(from_arr.iter().map(|(k, v)| (*v, *k)));
    println!("{:?}", bt);
    let chars_rev: Vec<char> = "héllo".chars().rev().collect();
    let back: String = chars_rev.iter().collect();
    println!("{:?} {}", chars_rev, back);
    let cow: std::borrow::Cow<str> = ["a", "b"].concat().into();
    println!("{}", cow);
    let vv: Vec<Vec<u8>> = (1..4).map(|n| (0..n).collect()).collect();
    println!("{:?}", vv);
    let ranges: Vec<std::ops::Range<usize>> = (0..3).map(|i| i * 2..i * 2 + 2).collect();
    println!("{:?}", ranges);
    let nested: BTreeMap<&str, BTreeMap<char, usize>> = ["ab", "ba", "aa"]
        .iter()
        .map(|w| (*w, w.chars().fold(BTreeMap::new(), |mut m, c| { *m.entry(c).or_insert(0) += 1; m })))
        .collect();
    println!("{:?}", nested);
    let opt_str: Option<String> = vec![Some("a"), Some("b")].into_iter().collect();
    println!("{:?}", opt_str);
    let res_hm: Result<HashMap<i32, i32>, String> = (1..4).map(|i| if i < 5 { Ok((i, i * 2)) } else { Err(format!("bad {}", i)) }).collect();
    let mut rv: Vec<_> = res_hm.unwrap().into_iter().collect();
    rv.sort_unstable_by(|a, b| b.cmp(a));
    println!("{:?}", rv);
    let unit: Result<(), String> = ["1", "2"].iter().map(|s| s.parse::<i32>().map(|_| ()).map_err(|e| e.to_string())).collect();
    println!("{:?}", unit);
}
