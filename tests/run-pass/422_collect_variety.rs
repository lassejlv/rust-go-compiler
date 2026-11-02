// Collecting into many collection types: HashMap (printed sorted), BTreeMap, BTreeSet, HashSet,
// VecDeque, String (from chars and &str), Box<[T]>, Result<Vec,_>, Option<Vec>, and extend.
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, LinkedList, VecDeque};

fn sorted_map<V: std::fmt::Debug>(m: &HashMap<String, V>) -> String {
    let mut keys: Vec<&String> = m.keys().collect();
    keys.sort();
    keys.iter().map(|k| format!("{}={:?}", k, m[*k])).collect::<Vec<_>>().join(", ")
}

fn main() {
    let text = "the cat sat on the mat the end";

    let freq: HashMap<String, usize> = text.split_whitespace().fold(HashMap::new(), |mut m, w| {
        *m.entry(w.to_string()).or_insert(0) += 1;
        m
    });
    println!("freq: {}", sorted_map(&freq));

    let by_len: BTreeMap<usize, Vec<&str>> = text.split(' ').fold(BTreeMap::new(), |mut m, w| {
        m.entry(w.len()).or_insert_with(Vec::new).push(w);
        m
    });
    println!("by_len: {:?}", by_len);

    let pairs: HashMap<String, i32> = vec![("b", 2), ("a", 1), ("c", 3), ("a", 10)].into_iter().map(|(k, v)| (k.to_string(), v)).collect();
    println!("pairs: {}", sorted_map(&pairs));

    let uniq: BTreeSet<char> = text.chars().filter(|c| c.is_alphabetic()).collect();
    println!("uniq: {}", uniq.iter().collect::<String>());
    let hs: HashSet<&str> = text.split(' ').collect();
    let mut hv: Vec<_> = hs.into_iter().collect();
    hv.sort_unstable();
    println!("hashset sorted: {:?}", hv);

    let dq: VecDeque<i32> = (1..=5).collect();
    let mut dq2 = dq.clone();
    dq2.rotate_left(2);
    dq2.push_front(0);
    println!("deque {:?} {:?} front={:?} back={:?}", dq, dq2, dq2.front(), dq2.back());

    let s1: String = vec!['h', 'i', '!'].into_iter().collect();
    let s2: String = vec!["ab", "cd", "ef"].into_iter().collect();
    let s3: String = vec![String::from("x"), String::from("yz")].into_iter().collect();
    let s4 = text.split(' ').map(|w| w.chars().rev().collect::<String>()).collect::<Vec<_>>().join("_");
    println!("{} {} {} {}", s1, s2, s3, s4);

    let boxed: Box<[u16]> = (0..4u16).map(|x| x << 4).collect();
    println!("boxed {:?} len {}", boxed, boxed.len());

    let ok: Result<Vec<i32>, String> = ["1", "2", "3"].iter().map(|s| s.parse::<i32>().map_err(|e| e.to_string())).collect();
    let bad: Result<Vec<i32>, String> = ["1", "zz", "3"].iter().map(|s| s.parse::<i32>().map_err(|e| e.to_string())).collect();
    println!("{:?} {:?}", ok, bad);

    let some: Option<Vec<char>> = ["a", "b"].iter().map(|s| s.chars().next()).collect();
    let none: Option<Vec<char>> = ["a", ""].iter().map(|s| s.chars().next()).collect();
    println!("{:?} {:?}", some, none);

    let heap: BinaryHeap<(u8, &str)> = vec![(3, "c"), (9, "z"), (1, "a"), (5, "m")].into_iter().collect();
    println!("heap sorted {:?}", heap.into_sorted_vec());
    let ll: LinkedList<i32> = (1..4).rev().collect();
    println!("linked {:?}", ll);

    let mut v: Vec<i32> = Vec::new();
    v.extend([1, 2, 3].iter());
    v.extend(vec![4, 5]);
    v.extend(Some(6));
    v.extend(None::<i32>);
    println!("extended {:?}", v);

    let mut s = String::from("start");
    s.extend(['-', '>']);
    s.extend(["a", "b"].iter().copied());
    s.extend(vec![String::from("C")]);
    println!("{}", s);

    let mut bm: BTreeMap<&str, i32> = BTreeMap::new();
    bm.extend([("z", 26), ("a", 1)]);
    bm.extend(vec![("m", 13)].into_iter());
    println!("{:?} first={:?} last={:?}", bm, bm.first_key_value(), bm.last_key_value());
    let range: Vec<(&&str, &i32)> = bm.range("b".."n").collect();
    println!("range {:?}", range);

    let grouped: BTreeMap<bool, Vec<i32>> = (1..=8).fold(BTreeMap::new(), |mut m, x| {
        m.entry(x % 3 == 0).or_insert_with(Vec::new).push(x);
        m
    });
    println!("{:?}", grouped);

    let inverted: BTreeMap<i32, String> = pairs.iter().map(|(k, v)| (*v, k.clone())).collect();
    println!("inverted {:?}", inverted);
    let counts: BTreeMap<char, usize> = "mississippi".chars().fold(BTreeMap::new(), |mut m, c| {
        *m.entry(c).or_default() += 1;
        m
    });
    println!("{:?}", counts);
}
