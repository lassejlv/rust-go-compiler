// BinaryHeap: max/min heap via Reverse, custom Ord, peek_mut, into_sorted_vec, k-smallest, merge k lists
use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;

#[derive(Debug, PartialEq, Eq)]
struct Task {
    pri: u8,
    seq: u32,
    name: String,
}
impl Ord for Task {
    fn cmp(&self, o: &Self) -> Ordering {
        self.pri.cmp(&o.pri).then_with(|| o.seq.cmp(&self.seq))
    }
}
impl PartialOrd for Task {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

#[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
struct Dist(f64);
impl Eq for Dist {}
impl Ord for Dist {
    fn cmp(&self, o: &Self) -> Ordering {
        self.0.total_cmp(&o.0)
    }
}

fn k_smallest(v: &[i32], k: usize) -> Vec<i32> {
    let mut h = BinaryHeap::new();
    for &x in v {
        h.push(x);
        if h.len() > k {
            h.pop();
        }
    }
    h.into_sorted_vec()
}

fn merge_sorted(lists: Vec<Vec<i32>>) -> Vec<i32> {
    let mut h = BinaryHeap::new();
    for (li, l) in lists.iter().enumerate() {
        if let Some(&x) = l.first() {
            h.push(Reverse((x, li, 0usize)));
        }
    }
    let mut out = vec![];
    while let Some(Reverse((x, li, i))) = h.pop() {
        out.push(x);
        if let Some(&y) = lists[li].get(i + 1) {
            h.push(Reverse((y, li, i + 1)));
        }
    }
    out
}

fn median_stream(v: &[i32]) -> Vec<f64> {
    let mut lo: BinaryHeap<i32> = BinaryHeap::new();
    let mut hi: BinaryHeap<Reverse<i32>> = BinaryHeap::new();
    let mut out = vec![];
    for &x in v {
        if lo.peek().map_or(true, |&m| x <= m) {
            lo.push(x);
        } else {
            hi.push(Reverse(x));
        }
        if lo.len() > hi.len() + 1 {
            hi.push(Reverse(lo.pop().unwrap()));
        } else if hi.len() > lo.len() {
            lo.push(hi.pop().unwrap().0);
        }
        let m = if lo.len() > hi.len() { *lo.peek().unwrap() as f64 } else { (*lo.peek().unwrap() as f64 + hi.peek().unwrap().0 as f64) / 2.0 };
        out.push(m);
    }
    out
}

fn main() {
    let mut h = BinaryHeap::from(vec![5, 1, 8, 3, 9, 2]);
    println!("peek={:?} len={}", h.peek(), h.len());
    if let Some(mut top) = h.peek_mut() {
        *top = 0;
    }
    println!("after peek_mut peek={:?}", h.peek());
    let mut popped = vec![];
    while let Some(x) = h.pop() {
        popped.push(x);
    }
    println!("{:?}", popped);
    let mut mh: BinaryHeap<Reverse<&str>> = ["pear", "apple", "fig", "banana"].into_iter().map(Reverse).collect();
    mh.push(Reverse("aardvark"));
    print!("min order:");
    while let Some(Reverse(s)) = mh.pop() {
        print!(" {}", s);
    }
    println!();
    let mut tasks = BinaryHeap::new();
    for (i, (p, n)) in [(2, "write"), (5, "fix"), (2, "read"), (9, "deploy"), (5, "test")].iter().enumerate() {
        tasks.push(Task { pri: *p, seq: i as u32, name: n.to_string() });
    }
    while let Some(t) = tasks.pop() {
        print!("{}({}) ", t.name, t.pri);
    }
    println!();
    let mut dh: BinaryHeap<Reverse<Dist>> = [3.5, -1.25, 0.0, 2.0].iter().map(|&d| Reverse(Dist(d))).collect();
    dh.push(Reverse(Dist(-7.0)));
    println!("{:?}", std::iter::from_fn(|| dh.pop().map(|r| r.0 .0)).collect::<Vec<_>>());
    println!("{:?}", k_smallest(&[9, 4, 7, 1, 8, 2, 6], 3));
    println!("{:?}", merge_sorted(vec![vec![1, 4, 9], vec![2, 3, 10, 11], vec![], vec![0, 5]]));
    println!("{:?}", median_stream(&[5, 15, 1, 3, 8, 7, 9, 10]));
    let mut a = BinaryHeap::from([1, 3]);
    let mut b = BinaryHeap::from([2, 4]);
    a.append(&mut b);
    println!("{:?} {}", a.clone().into_sorted_vec(), b.is_empty());
    let v = a.into_vec();
    println!("len {}", v.len());
    let mut h2: BinaryHeap<(u32, Reverse<String>)> = BinaryHeap::new();
    h2.push((1, Reverse("b".into())));
    h2.push((1, Reverse("a".into())));
    h2.push((0, Reverse("z".into())));
    println!("{:?}", h2.into_sorted_vec());
    let mut h3: BinaryHeap<i32> = (1..=10).collect();
    h3.retain(|x| x % 3 == 0);
    println!("{:?}", h3.into_sorted_vec());
}
