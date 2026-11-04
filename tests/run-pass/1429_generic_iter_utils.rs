use std::collections::{BTreeMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;

fn stats<I>(items: I) -> (usize, u64, Vec<u64>)
where
    I: IntoIterator,
    I::Item: Into<u64> + Copy,
    I::IntoIter: Clone,
{
    let it = items.into_iter();
    let n = it.clone().count();
    let total: u64 = it.clone().map(Into::into).sum();
    let evens = it.clone().map(|x| x.into()).filter(|x: &u64| x % 2 == 0).collect();
    (n, total, evens)
}

fn windows_sum<'a, T>(xs: &'a [T], k: usize) -> impl Iterator<Item = T> + 'a
where T: Copy + std::iter::Sum<T> + 'a {
    xs.windows(k).map(|w| w.iter().copied().sum())
}

fn uniq_by<I, K, F>(it: I, key: F) -> impl Iterator<Item = I::Item>
where I: Iterator, K: Hash + Eq, F: Fn(&I::Item) -> K {
    let mut seen = HashSet::new();
    it.filter(move |x| seen.insert(key(x)))
}

fn histogram<I>(it: I) -> BTreeMap<I::Item, usize> where I: IntoIterator, I::Item: Ord {
    let mut m = BTreeMap::new();
    for x in it { *m.entry(x).or_insert(0) += 1; }
    m
}

fn pipeline<I>(it: I) -> Vec<String>
where I: IntoIterator<Item = (String, i32)>, I::IntoIter: DoubleEndedIterator + ExactSizeIterator {
    let it = it.into_iter();
    let n = it.len();
    it.rev()
        .enumerate()
        .filter(|(_, (_, v))| *v >= 0)
        .map(|(i, (k, v))| (i, k, v))
        .flat_map(|(i, k, v)| std::iter::repeat(k).take(v as usize).map(move |s| format!("{}{}", s, i)))
        .chain(std::iter::once(format!("n={}", n)))
        .collect()
}

fn transpose<T: Clone + Debug>(rows: Vec<Vec<T>>) -> Vec<Vec<T>> {
    let w = rows.iter().map(Vec::len).max().unwrap_or(0);
    (0..w).map(|c| rows.iter().filter_map(|r| r.get(c).cloned()).collect()).collect()
}

fn cartesian<A: Clone, B: Clone>(a: &[A], b: &[B]) -> Vec<(A, B)> {
    a.iter().flat_map(|x| b.iter().map(move |y| (x.clone(), y.clone()))).collect()
}

fn group_runs<T: PartialEq + Clone>(xs: &[T]) -> Vec<(T, usize)> {
    xs.iter().fold(Vec::new(), |mut acc: Vec<(T, usize)>, x| {
        match acc.last_mut() { Some((y, n)) if y == x => *n += 1, _ => acc.push((x.clone(), 1)) }
        acc
    })
}

fn main() {
    println!("{:?}", stats(vec![1u8, 2, 3, 4]));
    println!("{:?}", stats([10u32, 15].iter().copied()));
    println!("{:?}", windows_sum(&[1, 2, 3, 4, 5], 3).collect::<Vec<i32>>());
    println!("{:?}", windows_sum(&[0.5, 1.5], 1).collect::<Vec<f64>>());
    println!("{:?}", uniq_by("hello world".chars(), |c| c.to_ascii_lowercase()).collect::<String>());
    println!("{:?}", uniq_by(vec![(1, 'a'), (2, 'b'), (1, 'c')].into_iter(), |p| p.0).collect::<Vec<_>>());
    println!("{:?}", histogram("mississippi".chars()));
    println!("{:?}", histogram(vec![3, 1, 3]));
    println!("{:?}", pipeline(vec![("a".to_string(), 2), ("b".to_string(), -1), ("c".to_string(), 1)]));
    println!("{:?}", transpose(vec![vec![1, 2, 3], vec![4], vec![5, 6]]));
    println!("{:?}", cartesian(&[1, 2], &["x", "y"]));
    println!("{:?}", group_runs(&['a', 'a', 'b', 'a', 'a', 'a']));
}
