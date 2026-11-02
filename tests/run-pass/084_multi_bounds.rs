use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::hash::Hash;

fn describe_all<T>(items: &[T]) -> String
where
    T: Display + PartialOrd + Copy,
{
    let mut max = items[0];
    let mut min = items[0];
    for &i in items {
        if i > max {
            max = i;
        }
        if i < min {
            min = i;
        }
    }
    format!("{} items, min {}, max {}", items.len(), min, max)
}

fn count_by<T, K, F>(items: &[T], key: F) -> Vec<(K, usize)>
where
    K: Hash + Eq + Ord + Clone,
    F: Fn(&T) -> K,
{
    let mut m: HashMap<K, usize> = HashMap::new();
    for it in items {
        *m.entry(key(it)).or_insert(0) += 1;
    }
    let mut out: Vec<(K, usize)> = m.into_iter().collect();
    out.sort();
    out
}

fn compare_and_show<A, B>(a: A, b: B) -> String
where
    A: Debug + PartialEq<B>,
    B: Debug,
{
    format!("{:?} == {:?} ? {}", a, b, a == b)
}

fn dedup_sorted<T: Ord + Clone + Debug>(mut v: Vec<T>) -> Vec<T> {
    v.sort();
    v.dedup();
    v
}

struct Wrapper<T> {
    inner: Vec<T>,
}

impl<T> Wrapper<T>
where
    T: Clone + Display,
{
    fn joined(&self, sep: &str) -> String {
        self.inner.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(sep)
    }
    fn doubled(&self) -> Wrapper<T> {
        let mut v = self.inner.clone();
        v.extend(self.inner.iter().cloned());
        Wrapper { inner: v }
    }
}

fn main() {
    println!("{}", describe_all(&[3, 9, -2, 7]));
    println!("{}", describe_all(&[2.5, 0.5]));
    println!("{}", describe_all(&['q', 'b', 'x']));
    let words = ["apple", "avocado", "banana", "blueberry", "cherry", "apricot"];
    println!("{:?}", count_by(&words, |w| w.chars().next().unwrap()));
    println!("{:?}", count_by(&words, |w| w.len()));
    println!("{:?}", count_by(&[1, 2, 3, 4, 5, 6, 7], |n| n % 3));
    println!("{}", compare_and_show(String::from("a"), "a"));
    println!("{}", compare_and_show(1, 2));
    println!("{:?}", dedup_sorted(vec![5, 3, 5, 1, 3]));
    println!("{:?}", dedup_sorted(vec!["b", "a", "b"]));
    let w = Wrapper { inner: vec![1, 2, 3] };
    println!("{}", w.joined("-"));
    println!("{}", w.doubled().joined(","));
}
