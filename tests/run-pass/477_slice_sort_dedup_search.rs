// Slice algorithms: sort/sort_unstable/sort_by_key/sort_by_cached_key stability, reverse sorts,
// binary_search (Ok/Err insertion points), binary_search_by_key, partition_point, dedup/dedup_by_key/
// dedup_by, retain/retain_mut, select_nth_unstable, fill/fill_with, copy_within, swap_with_slice,
// contains/iter().rev().position, and merging sorted runs.
#[derive(Debug, Clone)]
struct Rec {
    name: &'static str,
    age: u32,
    score: i32,
}

fn merge(a: &[i32], b: &[i32]) -> Vec<i32> {
    let (mut i, mut j) = (0, 0);
    let mut out = Vec::with_capacity(a.len() + b.len());
    while i < a.len() && j < b.len() {
        if a[i] <= b[j] {
            out.push(a[i]);
            i += 1;
        } else {
            out.push(b[j]);
            j += 1;
        }
    }
    out.extend_from_slice(&a[i..]);
    out.extend_from_slice(&b[j..]);
    out
}

fn main() {
    let recs = vec![
        Rec { name: "ann", age: 30, score: 90 },
        Rec { name: "bob", age: 25, score: 85 },
        Rec { name: "cat", age: 30, score: 70 },
        Rec { name: "dan", age: 25, score: 95 },
        Rec { name: "eve", age: 35, score: 85 },
        Rec { name: "fay", age: 30, score: 85 },
    ];
    let mut by_age = recs.clone();
    by_age.sort_by_key(|r| r.age); // stable
    println!("{:?}", by_age.iter().map(|r| r.name).collect::<Vec<_>>());
    let mut by_score_desc = recs.clone();
    by_score_desc.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.name.cmp(b.name)));
    println!("{:?}", by_score_desc.iter().map(|r| (r.name, r.score)).collect::<Vec<_>>());
    let mut cached = recs.clone();
    cached.sort_by_cached_key(|r| format!("{:03}{}", 100 - r.score, r.name));
    println!("{:?}", cached.iter().map(|r| r.name).collect::<Vec<_>>());
    let mut rev = recs.clone();
    rev.sort_by_key(|r| std::cmp::Reverse((r.age, r.name)));
    println!("{:?}", rev.iter().map(|r| r.name).collect::<Vec<_>>());

    let mut v = vec![5, 3, 9, 1, 3, 7, 9, 2, 8, 3];
    v.sort_unstable();
    println!("{:?}", v);
    for x in [3, 4, 0, 10, 9] {
        println!("search {} -> {:?} pp(<x)={}", x, v.binary_search(&x).map(|_| "found"), v.partition_point(|&e| e < x));
    }
    let r = by_age.binary_search_by_key(&30, |r| r.age).is_ok();
    let ins = by_age.binary_search_by_key(&28, |r| r.age);
    println!("{} {:?}", r, ins);
    let pos = v.binary_search_by(|e| e.cmp(&7).then(std::cmp::Ordering::Less));
    println!("{:?}", pos);

    let mut d = v.clone();
    d.dedup();
    println!("dedup {:?}", d);
    let mut words = vec!["apple", "avocado", "banana", "blueberry", "cherry", "apricot"];
    words.dedup_by_key(|w| w.chars().next().unwrap());
    println!("{:?}", words);
    let mut nums = vec![1, 2, 4, 7, 8, 9, 15, 16];
    nums.dedup_by(|a, b| *a - *b == 1);
    println!("{:?}", nums);

    let mut r = (1..=12).collect::<Vec<i32>>();
    r.retain(|x| x % 3 != 0);
    r.retain_mut(|x| {
        *x *= 10;
        *x != 50
    });
    println!("{:?}", r);

    let mut sel = vec![9, 1, 8, 2, 7, 3, 6, 4, 5];
    let (_, median, _) = sel.select_nth_unstable(4);
    println!("median {}", median);
    let mut top = vec![15, 3, 99, 42, 7, 64, 23];
    top.select_nth_unstable_by(2, |a, b| b.cmp(a));
    let mut top3 = top[..3].to_vec();
    top3.sort();
    println!("top3 {:?}", top3);

    let mut f = [0u8; 8];
    f[..3].fill(7);
    let mut c = 0;
    f[3..].fill_with(|| {
        c += 2;
        c
    });
    println!("{:?}", f);
    f.copy_within(0..4, 4);
    println!("{:?}", f);
    let (a, b) = f.split_at_mut(4);
    a.swap_with_slice(b);
    a.reverse();
    println!("{:?}", f);
    let mut g = [1, 2, 3, 4, 5, 6];
    g.copy_within(2.., 0);
    println!("{:?}", g);
    g.clone_from_slice(&[6, 5, 4, 3, 2, 1]);
    println!("{:?} {:?}", g, g.iter().rev().position(|&x| x == 5));

    println!("{:?}", merge(&[1, 4, 4, 9], &[0, 4, 5, 10, 11]));
    let mut runs = vec![vec![3, 8], vec![1, 9, 12], vec![2], vec![]];
    while runs.len() > 1 {
        let a = runs.remove(0);
        let b = runs.remove(0);
        runs.push(merge(&a, &b));
    }
    println!("{:?}", runs[0]);
    let mut fl = vec![2.5, -1.0, 9.75, 0.0, -3.5];
    fl.sort_by(f64::total_cmp);
    println!("{:?} {:?}", fl, fl.iter().cloned().fold(f64::NAN, f64::max));
    println!("{} {}", v.is_sorted(), v.iter().rev().is_sorted());
    let rotated_min = [4, 5, 6, 1, 2, 3].partition_point(|&x| x >= 4);
    println!("rotation point {}", rotated_min);
}
