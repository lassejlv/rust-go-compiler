use std::fmt::Display;

// impl Trait in argument position
fn join_all(items: impl IntoIterator<Item = impl Display>, sep: &str) -> String {
    let mut out = String::new();
    for (i, it) in items.into_iter().enumerate() {
        if i > 0 {
            out.push_str(sep);
        }
        out.push_str(&it.to_string());
    }
    out
}

// impl Trait in return position: iterators
fn evens(limit: u32) -> impl Iterator<Item = u32> + Clone {
    (0..limit).filter(|n| n % 2 == 0)
}

fn squares_of<'a>(xs: &'a [i64]) -> impl Iterator<Item = i64> + 'a {
    xs.iter().map(|x| x * x)
}

fn counter(start: i32) -> impl FnMut() -> i32 {
    let mut c = start;
    move || {
        c += 1;
        c
    }
}

fn make_adder(n: i32) -> impl Fn(i32) -> i32 + Copy {
    move |x| x + n
}

// impl Trait in associated-type-like position via trait returning impl (RPITIT)
trait Container {
    fn items(&self) -> impl Iterator<Item = &str>;
    fn count_long(&self, min: usize) -> usize {
        self.items().filter(|s| s.len() >= min).count()
    }
}

struct Words(Vec<String>);
struct Csv(String);

impl Container for Words {
    fn items(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(|s| s.as_str())
    }
}

impl Container for Csv {
    fn items(&self) -> impl Iterator<Item = &str> {
        self.0.split(',').map(str::trim).filter(|s| !s.is_empty())
    }
}

fn summarize(c: &impl Container) -> String {
    join_all(c.items().map(|s| s.to_uppercase()), "|")
}

// Nested impl Trait returning closures that return iterators
fn chunker(size: usize) -> impl Fn(&[u8]) -> Vec<Vec<u8>> {
    move |data| data.chunks(size).map(|c| c.to_vec()).collect()
}

// Return-position impl Trait with conditional branches unified via Box
fn pick(desc: bool) -> Box<dyn Iterator<Item = u8>> {
    if desc {
        Box::new((1..=5).rev())
    } else {
        Box::new(1..=5)
    }
}

// impl Trait + generic param mix
fn tally<K: Ord + Clone>(keys: impl Iterator<Item = K>) -> Vec<(K, usize)> {
    let mut v: Vec<(K, usize)> = Vec::new();
    for k in keys {
        match v.binary_search_by(|(x, _)| x.cmp(&k)) {
            Ok(i) => v[i].1 += 1,
            Err(i) => v.insert(i, (k, 1)),
        }
    }
    v
}

fn main() {
    println!("{}", join_all(vec![1, 2, 3], ", "));
    println!("{}", join_all(["a", "b"], "-"));
    println!("{}", join_all(evens(10), "+"));
    let e = evens(7);
    let e2 = e.clone();
    println!("{} {}", e.count(), e2.sum::<u32>());
    let xs = [3i64, -4, 5];
    println!("{:?}", squares_of(&xs).collect::<Vec<_>>());

    let mut c = counter(10);
    println!("{} {} {}", c(), c(), c());
    let add5 = make_adder(5);
    let copy = add5;
    println!("{} {}", add5(1), copy(100));

    let w = Words(vec!["apple".into(), "fig".into(), "banana".into()]);
    let csv = Csv(" x , yy,, zzz ,".into());
    println!("{} long={}", summarize(&w), w.count_long(4));
    println!("{} long={}", summarize(&csv), csv.count_long(2));

    let ch = chunker(3);
    println!("{:?}", ch(b"abcdefgh"));
    println!("{:?} {:?}", pick(true).collect::<Vec<_>>(), pick(false).collect::<Vec<_>>());
    println!("{:?}", tally("mississippi".chars()));
    println!("{:?}", tally(vec!["b", "a", "b", "c", "a", "b"].into_iter()));
}
