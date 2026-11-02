use std::collections::HashSet;
use std::hash::Hash;

fn sum_iter<I>(it: I) -> i64
where
    I: IntoIterator<Item = i64>,
{
    let mut s = 0;
    for x in it {
        s += x;
    }
    s
}

fn average<I: Iterator<Item = f64>>(it: I) -> Option<f64> {
    let mut n = 0;
    let mut total = 0.0;
    for x in it {
        n += 1;
        total += x;
    }
    if n == 0 { None } else { Some(total / n as f64) }
}

fn unique<T, I>(it: I) -> Vec<T>
where
    T: Eq + Hash + Clone,
    I: IntoIterator<Item = T>,
{
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for x in it {
        if seen.insert(x.clone()) {
            out.push(x);
        }
    }
    out
}

fn pairwise<T: Copy>(v: &[T]) -> impl Iterator<Item = (T, T)> + '_ {
    v.windows(2).map(|w| (w[0], w[1]))
}

fn apply_all<T, F>(items: &mut [T], f: F)
where
    F: Fn(&T) -> T,
{
    for x in items.iter_mut() {
        *x = f(x);
    }
}

fn generic_max<T: PartialOrd + Copy, I: IntoIterator<Item = T>>(it: I) -> Option<T> {
    let mut best: Option<T> = None;
    for x in it {
        best = match best {
            None => Some(x),
            Some(b) if x > b => Some(x),
            keep => keep,
        };
    }
    best
}

fn main() {
    println!("sum vec = {}", sum_iter(vec![1, 2, 3, 4]));
    println!("sum range = {}", sum_iter(1..=100));
    println!("sum mapped = {}", sum_iter([1i64, 2, 3].iter().map(|x| x * x)));
    println!("avg = {:?}", average(vec![1.0, 2.0, 4.5].into_iter()));
    println!("avg empty = {:?}", average(std::iter::empty()));
    println!("unique = {:?}", unique(vec![3, 1, 3, 2, 1, 4]));
    println!("unique chars = {:?}", unique("mississippi".chars()));
    let v = [1, 4, 9, 16];
    let p: Vec<(i32, i32)> = pairwise(&v).collect();
    println!("pairwise = {:?}", p);
    let mut nums = vec![1, 2, 3];
    apply_all(&mut nums, |x| x * 10 + 1);
    println!("apply_all = {:?}", nums);
    let mut strs = vec![String::from("a"), String::from("b")];
    apply_all(&mut strs, |s| s.repeat(3));
    println!("apply_all strs = {:?}", strs);
    println!("generic_max = {:?}", generic_max(vec![3, 8, 2]));
    println!("generic_max f = {:?}", generic_max([0.5, -1.0, 2.25]));
    println!("generic_max empty = {:?}", generic_max(Vec::<u8>::new()));
    println!("generic_max chars = {:?}", generic_max("hello".chars()));
}
