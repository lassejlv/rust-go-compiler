// Breadth test of VecDeque, LinkedList and BinaryHeap APIs.
use std::cmp::Reverse;
use std::collections::{BinaryHeap, LinkedList, VecDeque};

fn main() {
    let mut dq: VecDeque<i32> = VecDeque::new();
    for i in 0..5 {
        dq.push_back(i);
        dq.push_front(-i);
    }
    println!("deque: {:?} len {}", dq, dq.len());
    println!("front {:?} back {:?} [3] {:?} get(20) {:?}", dq.front(), dq.back(), dq[3], dq.get(20));
    dq.rotate_left(3);
    println!("rotate_left(3): {:?}", dq);
    dq.rotate_right(5);
    println!("rotate_right(5): {:?}", dq);
    let (a, b) = dq.as_slices();
    println!("as_slices total len {}", a.len() + b.len());
    dq.make_contiguous().sort();
    println!("make_contiguous sorted: {:?}", dq);
    println!("binary_search 3: {:?} 10: {:?}", dq.binary_search(&3), dq.binary_search(&10));
    println!("partition_point <0: {}", dq.partition_point(|&x| x < 0));
    dq.retain(|&x| x != 0);
    println!("retain !=0: {:?}", dq);
    dq.retain_mut(|x| { *x *= 2; *x > -6 });
    println!("retain_mut: {:?}", dq);
    let drained: Vec<i32> = dq.drain(1..3).collect();
    println!("drain(1..3): {:?} rest {:?}", drained, dq);
    let mut tail = dq.split_off(2);
    println!("split_off(2): {:?} | {:?}", dq, tail);
    dq.append(&mut tail);
    println!("append: {:?} tail empty {}", dq, tail.is_empty());
    dq.insert(1, 99);
    println!("insert(1, 99): {:?} remove(2) = {:?}", dq, dq.clone().remove(2));
    println!("swap_remove_back(0) {:?} -> {:?}", dq.clone().swap_remove_back(0), { let mut c = dq.clone(); c.swap_remove_back(0); c });
    println!("swap_remove_front(3) {:?}", { let mut c = dq.clone(); let r = c.swap_remove_front(3); (r, c) });
    dq.truncate(3);
    dq.resize(6, 7);
    println!("truncate/resize: {:?}", dq);
    dq.swap(0, 5);
    println!("swap: {:?} contains 7: {}", dq, dq.contains(&7));
    println!("iter rev: {:?}", dq.iter().rev().collect::<Vec<_>>());
    for x in dq.iter_mut() {
        *x += 1;
    }
    println!("range(1..4): {:?}", dq.range(1..4).collect::<Vec<_>>());
    if let Some(f) = dq.front_mut() {
        *f = -100;
    }
    println!("pop_front {:?} pop_back {:?} -> {:?}", dq.pop_front(), dq.pop_back(), dq);
    let v: Vec<i32> = dq.into_iter().collect();
    let dq2: VecDeque<i32> = VecDeque::from(v.clone());
    println!("roundtrip {:?} == {}", dq2, Vec::from(dq2.clone()) == v);
    let mut ring: VecDeque<u32> = VecDeque::with_capacity(4);
    for i in 0..10 {
        if ring.len() == 4 {
            ring.pop_front();
        }
        ring.push_back(i * i);
    }
    println!("ring buffer: {:?} sum {}", ring, ring.iter().sum::<u32>());
    let cmp = VecDeque::from([1, 2, 3]) == VecDeque::from([1, 2, 3]) && VecDeque::from([1, 2]) < VecDeque::from([1, 3]);
    println!("eq/ord: {}", cmp);

    // LinkedList
    let mut ll: LinkedList<&str> = LinkedList::new();
    ll.push_back("b");
    ll.push_back("c");
    ll.push_front("a");
    println!("list: {:?} len {} front {:?} back {:?}", ll, ll.len(), ll.front(), ll.back());
    let mut other: LinkedList<&str> = ["x", "y", "z"].into_iter().collect();
    ll.append(&mut other);
    println!("append: {:?} other {:?}", ll, other);
    let split = ll.split_off(4);
    println!("split_off(4): {:?} | {:?}", ll, split);
    println!("contains c: {} iter rev: {:?}", ll.contains(&"c"), ll.iter().rev().collect::<Vec<_>>());
    if let Some(b) = ll.back_mut() {
        *b = "X";
    }
    for s in ll.iter_mut() {
        if *s == "b" {
            *s = "B";
        }
    }
    println!("mutated: {:?} pop_front {:?} pop_back {:?}", ll.clone(), ll.pop_front(), ll.pop_back());
    ll.clear();
    println!("cleared: {:?} empty {}", ll, ll.is_empty());

    // BinaryHeap
    let mut h = BinaryHeap::from(vec![5, 1, 8, 3, 9, 2]);
    println!("peek {:?} len {}", h.peek(), h.len());
    h.push(7);
    if let Some(mut top) = h.peek_mut() {
        *top = 0; // sift down on drop
    }
    println!("after peek_mut: peek {:?}", h.peek());
    let mut out = vec![];
    while let Some(x) = h.pop() {
        out.push(x);
    }
    println!("pop order: {:?}", out);
    let mut minh: BinaryHeap<Reverse<(u32, &str)>> = BinaryHeap::new();
    for (p, n) in [(3, "c"), (1, "a"), (2, "b"), (1, "aa")] {
        minh.push(Reverse((p, n)));
    }
    println!("min-heap order: {:?}", std::iter::from_fn(|| minh.pop().map(|Reverse(x)| x)).collect::<Vec<_>>());
    let mut h2: BinaryHeap<i32> = [4, 4, 1, 9].into_iter().collect();
    let mut h3 = BinaryHeap::from([6, 2]);
    h2.append(&mut h3);
    println!("append len {} h3 empty {}", h2.len(), h3.is_empty());
    h2.retain(|&x| x != 4);
    println!("retain: {:?}", h2.clone().into_sorted_vec());
    println!("into_vec len {} sorted {:?}", h2.clone().into_vec().len(), h2.clone().into_sorted_vec());
    let drained: Vec<i32> = {
        let mut d: Vec<i32> = h2.drain().collect();
        d.sort();
        d
    };
    println!("drain: {:?} empty after {}", drained, h2.is_empty());
    let mut kth = BinaryHeap::new();
    for x in [15, 3, 99, 42, 7, 64, 23, 8] {
        kth.push(Reverse(x));
        if kth.len() > 3 {
            kth.pop();
        }
    }
    println!("3 largest: {:?}", kth.into_sorted_vec().into_iter().map(|Reverse(x)| x).collect::<Vec<_>>());
    let strs: BinaryHeap<String> = ["pear", "apple", "fig"].iter().map(|s| s.to_string()).collect();
    println!("{:?}", strs.into_sorted_vec());
}
