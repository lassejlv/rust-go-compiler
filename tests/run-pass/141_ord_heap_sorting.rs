use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
struct Task {
    priority: u32,
    name: String,
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
enum Value {
    Int(i64),
    Float(f64),
    Text(String),
    Pair(i32, i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Level {
    Low = 1,
    Mid = 5,
    High = 10,
}

#[derive(Debug, Eq, PartialEq)]
struct ByLen(String);

impl Ord for ByLen {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.len().cmp(&other.0.len()).then_with(|| other.0.cmp(&self.0))
    }
}
impl PartialOrd for ByLen {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn main() {
    let mut heap = BinaryHeap::new();
    heap.push(Task { priority: 2, name: "write".into() });
    heap.push(Task { priority: 5, name: "deploy".into() });
    heap.push(Task { priority: 2, name: "test".into() });
    heap.push(Task { priority: 1, name: "rest".into() });
    println!("peek {:?}", heap.peek().map(|t| &t.name));
    while let Some(t) = heap.pop() {
        println!("{} {}", t.priority, t.name);
    }

    let mut min_heap: BinaryHeap<Reverse<(u32, &str)>> = BinaryHeap::new();
    for (p, n) in [(3, "c"), (1, "a"), (2, "b"), (1, "aa")] {
        min_heap.push(Reverse((p, n)));
    }
    let order: Vec<_> = std::iter::from_fn(|| min_heap.pop().map(|Reverse(x)| x)).collect();
    println!("{:?}", order);

    let heap2: BinaryHeap<ByLen> = ["ccc", "a", "bb", "zz", "dddd"].iter().map(|s| ByLen(s.to_string())).collect();
    let sorted: Vec<String> = heap2.into_sorted_vec().into_iter().map(|b| b.0).collect();
    println!("{:?}", sorted);

    // Stability: sort_by_key keeps original order among equal keys.
    let people = vec![("amy", 30), ("bob", 25), ("cat", 30), ("dan", 25), ("eve", 35), ("fay", 30)];
    let mut by_age = people.clone();
    by_age.sort_by_key(|p| p.1);
    println!("{:?}", by_age);
    let mut by_age_desc = people.clone();
    by_age_desc.sort_by_key(|p| Reverse(p.1));
    println!("{:?}", by_age_desc);
    let mut by_two = people.clone();
    by_two.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    println!("{:?}", by_two);

    let mut vals = vec![
        Value::Text("b".into()),
        Value::Int(5),
        Value::Pair(1, 2),
        Value::Float(1.5),
        Value::Int(-3),
        Value::Text("a".into()),
        Value::Pair(1, 1),
    ];
    vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!("{:?}", vals);
    println!("{}", Value::Int(100) < Value::Float(0.0));
    println!("{:?}", Value::Float(f64::NAN).partial_cmp(&Value::Float(1.0)));
    println!("{:?}", Value::Float(2.0).partial_cmp(&Value::Float(1.0)));

    let mut levels = vec![Level::High, Level::Low, Level::Mid, Level::Low];
    levels.sort();
    println!("{:?} max={:?}", levels, levels.iter().max().unwrap());
    println!("{}", Level::Mid as i32 + Level::High as i32);
    println!("{:?}", Level::Low.cmp(&Level::High));
    println!("{:?}", (1, "b").cmp(&(1, "a")));
    println!("{:?}", [1, 2, 3][..].cmp(&[1, 2][..]));
    println!("{:?}", "abc".partial_cmp("abd"));
    println!("{:?} {:?}", 3.max(7), "x".min("a"));
    println!("{:?}", Ordering::Less.reverse());
    let mut fs: Vec<f64> = vec![2.5, -1.0, 9.75, 0.0];
    fs.sort_by(|a, b| b.total_cmp(a));
    println!("{:?}", fs);
}
