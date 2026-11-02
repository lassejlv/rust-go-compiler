// VecDeque and BinaryHeap operations: push/pop at both ends, rotate, make_contiguous, as_slices
// after wraparound, range/drain/retain/split_off/append, binary heap peek_mut modification,
// into_sorted_vec/into_vec, heap of custom Ord structs, a sliding-window maximum with a monotonic deque,
// and a k-way merge with a heap.
use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};

fn sliding_max(v: &[i32], k: usize) -> Vec<i32> {
    let mut dq: VecDeque<usize> = VecDeque::new();
    let mut out = vec![];
    for i in 0..v.len() {
        while dq.front().map_or(false, |&f| f + k <= i) {
            dq.pop_front();
        }
        while dq.back().map_or(false, |&b| v[b] <= v[i]) {
            dq.pop_back();
        }
        dq.push_back(i);
        if i + 1 >= k {
            out.push(v[*dq.front().unwrap()]);
        }
    }
    out
}

fn kway(lists: &[Vec<i32>]) -> Vec<i32> {
    let mut h = BinaryHeap::new();
    for (li, l) in lists.iter().enumerate() {
        if let Some(&x) = l.first() {
            h.push(Reverse((x, li, 0usize)));
        }
    }
    let mut out = vec![];
    while let Some(Reverse((x, li, i))) = h.pop() {
        out.push(x);
        if let Some(&nx) = lists[li].get(i + 1) {
            h.push(Reverse((nx, li, i + 1)));
        }
    }
    out
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Job {
    pri: u8,
    name: String,
}

fn main() {
    let mut d: VecDeque<i32> = VecDeque::with_capacity(4);
    d.push_back(1);
    d.push_back(2);
    d.push_front(0);
    d.push_front(-1);
    d.push_back(3);
    println!("{:?} len={} front={:?} back={:?}", d, d.len(), d.front(), d.back());
    d.rotate_left(2);
    println!("rotl {:?}", d);
    d.rotate_right(1);
    println!("rotr {:?} [2]={} get(9)={:?}", d, d[2], d.get(9));
    let (a, b) = d.as_slices();
    println!("slices total {}", a.len() + b.len());
    d.make_contiguous().sort();
    println!("sorted {:?} {:?}", d, d.as_slices().0);
    let drained: Vec<i32> = d.drain(1..3).collect();
    println!("drained {:?} left {:?}", drained, d);
    d.extend([10, 11, 12, 13]);
    d.retain(|x| x % 2 == 1 || *x > 11);
    println!("retained {:?}", d);
    let tail = d.split_off(2);
    println!("split {:?} {:?}", d, tail);
    let mut t = tail;
    d.append(&mut t);
    println!("append {:?} {}", d, t.is_empty());
    println!("range {:?}", d.range(1..).collect::<Vec<_>>());
    if let Some(x) = d.get_mut(0) {
        *x = 100;
    }
    d.swap(0, 1);
    println!("{:?} contains 100: {} pos {:?}", d, d.contains(&100), d.iter().position(|&x| x == 100));
    d.truncate(2);
    d.insert(1, 55);
    let before = format!("{:?}", d);
    println!("{} remove(0)={:?}", before, d.remove(0));
    let v: Vec<i32> = d.into_iter().rev().collect();
    println!("{:?}", v);

    let mut ring: VecDeque<char> = "abcdef".chars().collect();
    for _ in 0..3 {
        let c = ring.pop_front().unwrap();
        ring.push_back(c.to_ascii_uppercase());
    }
    println!("{}", ring.iter().collect::<String>());
    println!("{:?}", ring.iter().rev().step_by(2).collect::<String>());

    println!("{:?}", sliding_max(&[1, 3, -1, -3, 5, 3, 6, 7], 3));
    println!("{:?}", sliding_max(&[9, 8, 7, 6], 2));
    println!("{:?}", kway(&[vec![1, 4, 9], vec![2, 3, 10, 11], vec![], vec![0, 5]]));

    let mut h = BinaryHeap::from(vec![3, 1, 4, 1, 5, 9, 2, 6]);
    println!("peek {:?} len {}", h.peek(), h.len());
    if let Some(mut top) = h.peek_mut() {
        *top = 0; // sift down on drop
    }
    println!("peek after {:?}", h.peek());
    h.push(7);
    let mut popped = vec![];
    while popped.len() < 3 {
        popped.push(h.pop().unwrap());
    }
    println!("popped {:?}", popped);
    let mut rest = h.clone().into_vec();
    rest.sort();
    println!("rest {:?} sorted {:?}", rest, h.into_sorted_vec());

    let mut jobs = BinaryHeap::new();
    for (p, n) in [(2, "write"), (5, "deploy"), (2, "read"), (9, "hotfix"), (5, "build")] {
        jobs.push(Job { pri: p, name: n.into() });
    }
    let order: Vec<String> = std::iter::from_fn(|| jobs.pop()).map(|j| format!("{}:{}", j.pri, j.name)).collect();
    println!("{:?}", order);
    let mut minh: BinaryHeap<Reverse<String>> = ["pear", "apple", "fig"].iter().map(|s| Reverse(s.to_string())).collect();
    minh.retain(|Reverse(s)| s.len() > 3);
    println!("{:?}", minh.pop().map(|Reverse(s)| s));
    let mut a = BinaryHeap::from([1, 5]);
    let mut b = BinaryHeap::from([3, 7]);
    a.append(&mut b);
    println!("{:?} {}", a.into_sorted_vec(), b.len());
}
