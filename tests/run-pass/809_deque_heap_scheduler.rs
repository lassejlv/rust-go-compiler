use std::cmp::{Ordering, Reverse};
use std::collections::{BinaryHeap, VecDeque};

#[derive(Debug, PartialEq, Eq)]
struct Job { prio: u8, seq: u32, name: String, cost: u32 }

impl Ord for Job {
    fn cmp(&self, other: &Self) -> Ordering {
        self.prio.cmp(&other.prio).then_with(|| other.seq.cmp(&self.seq))
    }
}
impl PartialOrd for Job { fn partial_cmp(&self, o: &Self) -> Option<Ordering> { Some(self.cmp(o)) } }

fn round_robin(mut q: VecDeque<(char, u32)>, quantum: u32) -> Vec<(char, u32)> {
    let mut t = 0;
    let mut done = Vec::new();
    while let Some((id, rem)) = q.pop_front() {
        let run = rem.min(quantum);
        t += run;
        if rem > quantum { q.push_back((id, rem - quantum)); } else { done.push((id, t)); }
    }
    done
}

fn sliding_max(v: &[i32], k: usize) -> Vec<i32> {
    let mut dq: VecDeque<usize> = VecDeque::new();
    let mut out = Vec::new();
    for i in 0..v.len() {
        while dq.front().map_or(false, |&j| j + k <= i) { dq.pop_front(); }
        while dq.back().map_or(false, |&j| v[j] <= v[i]) { dq.pop_back(); }
        dq.push_back(i);
        if i + 1 >= k { out.push(v[*dq.front().unwrap()]); }
    }
    out
}

fn kth_smallest_merge(lists: &[Vec<i32>]) -> Vec<i32> {
    let mut heap = BinaryHeap::new();
    for (li, l) in lists.iter().enumerate() {
        if let Some(&x) = l.first() { heap.push(Reverse((x, li, 0usize))); }
    }
    let mut out = Vec::new();
    while let Some(Reverse((x, li, idx))) = heap.pop() {
        out.push(x);
        if let Some(&nx) = lists[li].get(idx + 1) { heap.push(Reverse((nx, li, idx + 1))); }
    }
    out
}

fn main() {
    let mut heap = BinaryHeap::new();
    let specs = [(2, "compile", 5), (5, "deploy", 2), (2, "lint", 1), (9, "hotfix", 3), (5, "test", 4), (1, "docs", 1), (9, "page", 1)];
    for (i, (p, n, c)) in specs.iter().enumerate() {
        heap.push(Job { prio: *p, seq: i as u32, name: n.to_string(), cost: *c });
    }
    println!("peek {:?}", heap.peek().map(|j| &j.name));
    let mut clock = 0;
    while let Some(job) = heap.pop() {
        clock += job.cost;
        println!("t={:>2} ran {:<8} prio={} seq={}", clock, job.name, job.prio, job.seq);
        if job.name == "deploy" {
            heap.push(Job { prio: 3, seq: 100, name: "verify".into(), cost: 2 });
        }
    }
    let mut mh: BinaryHeap<Reverse<u64>> = vec![50, 10, 40, 20, 30].into_iter().map(Reverse).collect();
    mh.push(Reverse(5));
    let firsts: Vec<u64> = std::iter::from_fn(|| mh.pop().map(|Reverse(x)| x)).take(3).collect();
    println!("min-heap firsts {:?} remaining len {}", firsts, mh.len());
    let sorted = mh.into_sorted_vec();
    println!("into_sorted_vec {:?}", sorted.iter().map(|r| r.0).collect::<Vec<_>>());
    let mut h2 = BinaryHeap::from(vec![3, 1, 4, 1, 5, 9, 2, 6]);
    let mut h3 = BinaryHeap::from([7, 8]);
    h2.append(&mut h3);
    println!("len {} top {:?} vec-sorted {:?}", h2.len(), h2.peek(), h2.clone().into_sorted_vec());
    if let Some(mut top) = h2.peek_mut() { *top = 0; }
    println!("after peek_mut top {:?}", h2.peek());
    h2.retain(|&x| x > 2);
    let mut drained: Vec<_> = h2.drain().collect();
    drained.sort();
    println!("drained sorted {:?}", drained);

    let q: VecDeque<_> = vec![('A', 7), ('B', 3), ('C', 5), ('D', 1)].into();
    println!("rr {:?}", round_robin(q, 2));
    println!("sliding max {:?}", sliding_max(&[1, 3, -1, -3, 5, 3, 6, 7, 2, 2, 1], 3));
    println!("merge {:?}", kth_smallest_merge(&[vec![1, 4, 9], vec![2, 3, 10, 11], vec![], vec![0, 5]]));

    let mut d: VecDeque<i32> = VecDeque::with_capacity(4);
    for i in 0..6 { if i % 2 == 0 { d.push_back(i) } else { d.push_front(i) } }
    println!("deque {:?} front {:?} back {:?} [2]={}", d, d.front(), d.back(), d[2]);
    d.rotate_left(2);
    println!("rotl {:?}", d);
    d.rotate_right(3);
    println!("rotr {:?}", d);
    let (a, b) = d.as_slices();
    println!("slices total {}", a.len() + b.len());
    d.make_contiguous().sort();
    println!("sorted {:?} contains3 {} pos {:?}", d, d.contains(&3), d.iter().position(|&x| x == 4));
    d.retain(|&x| x != 2);
    d.insert(1, 99);
    println!("{:?}", d);
    let r0 = d.remove(0);
    let r1 = d.swap_remove_back(0);
    println!("remove(0)={:?} swap_remove_back(0)={:?} -> {:?}", r0, r1, d);
    let tail = d.split_off(2);
    println!("{:?} {:?}", d, tail);
    d.extend(tail.iter().rev());
    d.truncate(4);
    let rv: Vec<_> = d.iter().rev().collect();
    println!("{:?} rev {:?}", d, rv);
    for x in d.iter_mut() { *x *= 10; }
    let drained: Vec<i32> = d.drain(1..).collect();
    println!("{:?} {:?}", d, drained);
    let mut pal: VecDeque<char> = "racecar".chars().collect();
    let mut is_pal = true;
    while pal.len() > 1 { if pal.pop_front() != pal.pop_back() { is_pal = false; } }
    println!("palindrome {}", is_pal);
    let rng: VecDeque<i32> = (1..=10).collect();
    println!("range iter {:?}", rng.range(3..7).collect::<Vec<_>>());
    println!("binary_search {:?} {:?}", rng.binary_search(&7), rng.binary_search(&70));
}
