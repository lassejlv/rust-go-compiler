use std::collections::BTreeMap;
use std::fmt::Debug;

fn group_sum<I, K, V>(iter: I) -> BTreeMap<K, V>
where
    I: IntoIterator<Item = (K, V)>,
    K: Ord,
    V: std::ops::AddAssign + Default + Copy,
{
    let mut m = BTreeMap::new();
    for (k, v) in iter {
        *m.entry(k).or_insert_with(V::default) += v;
    }
    m
}

fn print_all<I>(it: I)
where
    I: Iterator,
    I::Item: Debug,
{
    let v: Vec<String> = it.map(|x| format!("{:?}", x)).collect();
    println!("[{}]", v.join(", "));
}

fn sum_lengths<'a, I>(it: I) -> usize
where
    I: Iterator<Item = &'a str>,
{
    it.map(|s| s.len()).sum()
}

fn zip_with<A, B, C, IA, IB, F>(a: IA, b: IB, f: F) -> Vec<C>
where
    IA: IntoIterator<Item = A>,
    IB: IntoIterator<Item = B>,
    F: Fn(A, B) -> C,
{
    a.into_iter().zip(b).map(|(x, y)| f(x, y)).collect()
}

fn invert<K, V, I>(it: I) -> BTreeMap<V, Vec<K>>
where
    I: Iterator<Item = (K, V)>,
    V: Ord,
{
    let mut out: BTreeMap<V, Vec<K>> = BTreeMap::new();
    for (k, v) in it {
        out.entry(v).or_default().push(k);
    }
    out
}

trait Container {
    type Item;
    fn items(&self) -> Vec<Self::Item>;
}

struct Words(String);
impl Container for Words {
    type Item = String;
    fn items(&self) -> Vec<String> {
        self.0.split_whitespace().map(String::from).collect()
    }
}

struct Digits(u64);
impl Container for Digits {
    type Item = u8;
    fn items(&self) -> Vec<u8> {
        self.0.to_string().bytes().map(|b| b - b'0').collect()
    }
}

fn count_matching<C>(c: &C, target: &C::Item) -> usize
where
    C: Container,
    C::Item: PartialEq,
{
    c.items().iter().filter(|x| *x == target).count()
}

fn total<C: Container<Item = u8>>(c: &C) -> u32 {
    c.items().iter().map(|&d| d as u32).sum()
}

fn main() {
    let data = vec![("b", 2), ("a", 1), ("b", 5), ("c", 7), ("a", 10)];
    let m = group_sum(data);
    println!("{:?}", m);
    let fm = group_sum(vec![(1u8, 0.5f64), (2, 1.25), (1, 2.0)]);
    println!("{:?}", fm);
    print_all(vec![1, 2, 3].into_iter().map(|x| x * x));
    print_all("abc".chars());
    print_all(m.iter());
    println!("{}", sum_lengths(vec!["hello", "to", "you"].into_iter()));
    println!("{}", sum_lengths("a bb ccc".split(' ')));
    let z = zip_with(vec![1, 2, 3], vec![10, 20, 30, 40], |a, b| a * b);
    println!("{:?}", z);
    let z2 = zip_with("xyz".chars(), 1..4, |c, n| c.to_string().repeat(n));
    println!("{:?}", z2);
    let inv = invert(vec![("x", 1), ("y", 2), ("z", 1), ("w", 3)].into_iter());
    println!("{:?}", inv);
    let w = Words("the cat and the hat and the bat".to_string());
    println!("{}", count_matching(&w, &"the".to_string()));
    let d = Digits(1223334444);
    println!("{} {}", count_matching(&d, &4), total(&d));
}
