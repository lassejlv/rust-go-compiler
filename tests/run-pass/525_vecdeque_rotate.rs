// VecDeque: rotate, make_contiguous, as_slices, range, drain, split_off, sliding window max, josephus
use std::collections::VecDeque;

fn sliding_max(v: &[i32], k: usize) -> Vec<i32> {
    let mut dq: VecDeque<usize> = VecDeque::new();
    let mut out = vec![];
    for i in 0..v.len() {
        while dq.front().map_or(false, |&j| j + k <= i) {
            dq.pop_front();
        }
        while dq.back().map_or(false, |&j| v[j] <= v[i]) {
            dq.pop_back();
        }
        dq.push_back(i);
        if i + 1 >= k {
            out.push(v[*dq.front().unwrap()]);
        }
    }
    out
}

fn josephus(n: usize, k: usize) -> Vec<usize> {
    let mut q: VecDeque<usize> = (1..=n).collect();
    let mut order = vec![];
    while !q.is_empty() {
        q.rotate_left((k - 1) % q.len());
        order.push(q.pop_front().unwrap());
    }
    order
}

fn main() {
    let mut d: VecDeque<i32> = VecDeque::with_capacity(4);
    for i in 0..4 {
        d.push_back(i);
    }
    d.push_front(-1);
    d.push_front(-2);
    println!("{:?} front={:?} back={:?}", d, d.front(), d.back());
    d.rotate_left(2);
    println!("rl2 {:?}", d);
    d.rotate_right(3);
    println!("rr3 {:?}", d);
    let (a, b) = d.as_slices();
    println!("slices total {}", a.len() + b.len());
    let sl = d.make_contiguous();
    sl.sort();
    println!("sorted {:?}", sl);
    println!("{:?}", d.as_slices().1.is_empty());
    println!("{:?}", d.range(1..4).collect::<Vec<_>>());
    for x in d.range_mut(..2) {
        *x *= 100;
    }
    println!("{:?}", d);
    let drained: Vec<i32> = d.drain(1..3).collect();
    println!("drained {:?} left {:?}", drained, d);
    let tail = d.split_off(2);
    println!("{:?} | {:?}", d, tail);
    d.extend(tail.iter().rev());
    println!("{:?}", d);
    d.insert(1, 77);
    println!("{:?} removed={:?}", d, d.clone().remove(0));
    println!("{:?}", d.swap_remove_back(0));
    println!("{:?}", d);
    d.truncate(2);
    d.resize(5, 9);
    println!("{:?} contains9={}", d, d.contains(&9));
    println!("{:?}", d.iter().rev().collect::<Vec<_>>());
    println!("{:?} {:?}", d.get(1), d.get(10));
    d.swap(0, 4);
    println!("{:?}", d);
    d.retain(|&x| x != 9);
    println!("{:?}", d);
    println!("{:?}", sliding_max(&[1, 3, -1, -3, 5, 3, 6, 7], 3));
    println!("{:?}", josephus(7, 3));
    let mut bfs = VecDeque::from(vec![(0u32, 1u64)]);
    let mut seen = vec![];
    while let Some((depth, v)) = bfs.pop_front() {
        seen.push(v);
        if depth < 3 {
            bfs.push_back((depth + 1, v * 2));
            bfs.push_back((depth + 1, v * 2 + 1));
        }
    }
    println!("{:?}", seen);
    let mut ring: VecDeque<char> = "abcdef".chars().collect();
    let mut outs = vec![];
    for _ in 0..6 {
        ring.rotate_left(1);
        outs.push(ring.iter().collect::<String>());
    }
    println!("{:?}", outs);
    let v: Vec<char> = ring.into();
    println!("{:?}", v);
    let mut dq2: VecDeque<i32> = VecDeque::new();
    for i in 0..10 {
        if i % 2 == 0 { dq2.push_front(i) } else { dq2.push_back(i) }
    }
    println!("{:?}", dq2);
    println!("{:?}", dq2.iter().position(|&x| x == 5));
    dq2.make_contiguous().sort_by(|a, b| b.cmp(a));
    println!("{:?} {:?}", dq2, dq2.binary_search_by(|x| 5.cmp(x)));
    println!("{}", dq2 == VecDeque::from(vec![9, 8, 7, 6, 5, 4, 3, 2, 1, 0]));
}
