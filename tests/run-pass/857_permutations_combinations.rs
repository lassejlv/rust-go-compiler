struct Combinations { n: usize, k: usize, idx: Vec<usize>, done: bool }
impl Combinations { fn new(n: usize, k: usize) -> Self { Combinations { n, k, idx: (0..k).collect(), done: k > n } } }
impl Iterator for Combinations {
    type Item = Vec<usize>;
    fn next(&mut self) -> Option<Vec<usize>> {
        if self.done { return None; }
        let out = self.idx.clone();
        let mut i = self.k;
        loop {
            if i == 0 { self.done = true; break; }
            i -= 1;
            if self.idx[i] != i + self.n - self.k { self.idx[i] += 1; for j in i + 1..self.k { self.idx[j] = self.idx[j - 1] + 1; } break; }
        }
        Some(out)
    }
}

fn next_permutation<T: Ord>(v: &mut [T]) -> bool {
    if v.len() < 2 { return false; }
    let mut i = v.len() - 1;
    while i > 0 && v[i - 1] >= v[i] { i -= 1; }
    if i == 0 { v.reverse(); return false; }
    let mut j = v.len() - 1;
    while v[j] <= v[i - 1] { j -= 1; }
    v.swap(i - 1, j);
    v[i..].reverse();
    true
}

fn heap_perms<T: Clone>(k: usize, a: &mut Vec<T>, out: &mut Vec<Vec<T>>) {
    if k == 1 { out.push(a.clone()); return; }
    heap_perms(k - 1, a, out);
    for i in 0..k - 1 {
        if k % 2 == 0 { a.swap(i, k - 1); } else { a.swap(0, k - 1); }
        heap_perms(k - 1, a, out);
    }
}

fn queens(n: usize) -> (u64, Option<Vec<usize>>) {
    fn go(n: usize, row: usize, cols: u32, d1: u32, d2: u32, place: &mut Vec<usize>, first: &mut Option<Vec<usize>>) -> u64 {
        if row == n { if first.is_none() { *first = Some(place.clone()); } return 1; }
        let mut avail = !(cols | d1 | d2) & ((1 << n) - 1);
        let mut count = 0;
        while avail != 0 {
            let bit = avail & avail.wrapping_neg();
            avail ^= bit;
            place.push(bit.trailing_zeros() as usize);
            count += go(n, row + 1, cols | bit, (d1 | bit) << 1, (d2 | bit) >> 1, place, first);
            place.pop();
        }
        count
    }
    let mut first = None;
    let c = go(n, 0, 0, 0, 0, &mut vec![], &mut first);
    (c, first)
}

fn subsets_with_sum(v: &[i32], target: i32) -> Vec<Vec<i32>> {
    (0u32..1 << v.len()).filter_map(|mask| {
        let s: Vec<i32> = v.iter().enumerate().filter(|(i, _)| mask >> i & 1 == 1).map(|(_, &x)| x).collect();
        if s.iter().sum::<i32>() == target { Some(s) } else { None }
    }).collect()
}

fn partitions(n: u32, max: u32) -> Vec<Vec<u32>> {
    if n == 0 { return vec![vec![]]; }
    (1..=max.min(n)).rev().flat_map(|k| partitions(n - k, k).into_iter().map(move |mut p| { p.insert(0, k); p })).collect()
}

fn main() {
    let c: Vec<Vec<usize>> = Combinations::new(5, 3).collect();
    println!("C(5,3)={} {:?}", c.len(), &c[..4]);
    println!("C(4,0)={:?} C(2,3)={:?}", Combinations::new(4, 0).collect::<Vec<_>>(), Combinations::new(2, 3).count());
    let letters = ['a', 'b', 'c', 'd', 'e', 'f'];
    let words: Vec<String> = Combinations::new(6, 2).map(|ix| ix.iter().map(|&i| letters[i]).collect()).collect();
    println!("{}", words.join(","));
    let mut p = vec![1, 2, 2, 3];
    let mut all = vec![p.clone()];
    while next_permutation(&mut p) { all.push(p.clone()); }
    println!("{} distinct perms of 1223: {:?}", all.len(), all.iter().map(|v| v.iter().map(|d| d.to_string()).collect::<String>()).collect::<Vec<_>>());
    println!("after wrap: {:?}", p);
    let mut h = vec!['x', 'y', 'z'];
    let mut hp = vec![];
    heap_perms(3, &mut h, &mut hp);
    println!("{:?}", hp.iter().map(|v| v.iter().collect::<String>()).collect::<Vec<_>>());
    for n in [1, 4, 6, 8, 10] { let (c, f) = queens(n); println!("queens({}) = {} first {:?}", n, c, f); }
    println!("{:?}", subsets_with_sum(&[3, 34, 4, 12, 5, 2], 9));
    let parts = partitions(6, 6);
    println!("p(6)={} {:?}", parts.len(), parts);
    println!("p(20)={}", partitions(20, 20).len());
    let mut s = *b"dcba";
    let mut steps = 0; while next_permutation(&mut s) { steps += 1; }
    println!("{} {:?}", steps, std::str::from_utf8(&s).unwrap());
}
