use std::collections::VecDeque;

#[derive(Debug, Clone, PartialEq)]
struct Item { k: u8, v: i32 }

fn main() {
    let mut v: Vec<i32> = (1..=10).collect();
    let drained: Vec<i32> = v.drain(2..5).collect();
    println!("{:?} {:?}", drained, v);
    let spliced: Vec<i32> = v.splice(1..3, [100, 200, 300]).collect();
    println!("{:?} {:?}", spliced, v);
    v.retain_mut(|x| { *x += 1; *x % 2 == 0 });
    println!("{:?}", v);
    let mut w: Vec<i32> = (1..=12).collect();
    let evens: Vec<i32> = w.extract_if(.., |x| *x % 2 == 0).collect();
    println!("{:?} {:?}", evens, w);
    let tail = w.split_off(4);
    println!("{:?} {:?}", w, tail);
    w.extend_from_slice(&tail[..2]);
    w.extend_from_within(1..3);
    println!("{:?}", w);
    w.truncate(5);
    w.insert(0, -1);
    let before = w.clone();
    let sr = w.swap_remove(1);
    println!("{:?} swap_remove {} -> {:?}", before, sr, w);
    let mut items = vec![
        Item { k: 1, v: 10 }, Item { k: 1, v: 11 }, Item { k: 2, v: 20 },
        Item { k: 1, v: 12 }, Item { k: 3, v: 30 }, Item { k: 3, v: 31 },
    ];
    items.dedup_by_key(|i| i.k);
    println!("{:?}", items.iter().map(|i| i.v).collect::<Vec<_>>());
    items.dedup_by(|a, b| a.k == b.k + 2);
    println!("{:?}", items.iter().map(|i| (i.k, i.v)).collect::<Vec<_>>());
    let mut s = vec!["b", "a", "c"];
    s.resize(5, "z");
    println!("{:?}", s);
    s.resize_with(2, || "unused");
    println!("{:?}", s);
    let mut nested = vec![vec![1, 2], vec![3]];
    nested.iter_mut().for_each(|v| v.push(0));
    println!("{:?} {:?}", nested, nested.concat());
    let mut boxed: Box<[i32]> = vec![3, 1, 2].into_boxed_slice();
    boxed.sort();
    println!("{:?} {}", boxed, boxed.len());
    let back: Vec<i32> = boxed.into_vec();
    println!("{:?}", back.iter().rev().collect::<Vec<_>>());
    let mut dq: VecDeque<i32> = (1..=5).collect();
    dq.rotate_left(2);
    dq.push_front(0);
    let snapshot = dq.clone();
    println!("{:?} {:?}", snapshot, dq.make_contiguous());
    let (a, b) = dq.as_slices();
    println!("{} {}", a.len() + b.len(), dq.iter().rev().map(|x| x.to_string()).collect::<String>());
    let removed: Vec<i32> = dq.drain(1..3).collect();
    println!("{:?} {:?}", removed, dq);
    dq.retain(|&x| x != 5);
    println!("{:?} {:?} {:?}", dq, dq.front(), dq.back());
    let mut vv = vec![1, 2, 3, 4, 5, 6];
    let (l, r) = vv.split_at_mut(3);
    l[0] = r[2];
    r[0] = l[1];
    println!("{:?}", vv);
    if let [first, rest @ ..] = vv.as_mut_slice() {
        *first = rest.iter().sum();
    }
    println!("{:?}", vv);
    let mut cap = Vec::<u8>::with_capacity(10);
    cap.push(1);
    cap.shrink_to_fit();
    println!("cap>=len {}", cap.capacity() >= cap.len());
    println!("{:?}", vec![1, 2, 3].into_iter().rev().chain(std::iter::once(0)).collect::<Vec<_>>());
    let mut m = vec![1, 2, 3];
    m.clear();
    println!("{} {}", m.is_empty(), m.len());
    let mut ve = vec![5, 3, 8];
    if let Some(last) = ve.last_mut() { *last *= 2; }
    if let Some(first) = ve.first_mut() { *first -= 1; }
    let popped = ve.pop_if(|x| *x > 10);
    println!("{:?} {:?}", ve, popped);
}
