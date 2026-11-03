// impl Trait in argument and return position, iterator-returning functions, nested impl Trait
use std::fmt::Display;

fn evens_squared(limit: u32) -> impl Iterator<Item = u32> + Clone {
    (0..limit).filter(|n| n % 2 == 0).map(|n| n * n)
}

fn words<'a>(s: &'a str) -> impl Iterator<Item = &'a str> + 'a {
    s.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty())
}

fn numbered<T: Display>(it: impl IntoIterator<Item = T>) -> impl Iterator<Item = String> {
    it.into_iter().enumerate().map(|(i, x)| format!("{}:{}", i + 1, x))
}

fn counter(start: i32) -> impl FnMut() -> i32 {
    let mut c = start;
    move || {
        c += 1;
        c
    }
}

fn either_iter(flag: bool) -> Box<dyn Iterator<Item = i32>> {
    if flag {
        Box::new([1, 2, 3].into_iter())
    } else {
        Box::new((10..13).rev())
    }
}

fn pairs<I>(it: I) -> impl Iterator<Item = (I::Item, I::Item)>
where
    I: Iterator + Clone,
    I::Item: Clone,
{
    let a = it.clone();
    a.clone().enumerate().flat_map(move |(i, x)| {
        a.clone().skip(i + 1).map(move |y| (x.clone(), y))
    })
}

fn fib() -> impl Iterator<Item = u64> {
    std::iter::successors(Some((0u64, 1u64)), |&(a, b)| Some((b, a + b))).map(|p| p.0)
}

fn collatz(n: u64) -> impl Iterator<Item = u64> {
    std::iter::successors(Some(n), |&x| {
        if x == 1 {
            None
        } else if x % 2 == 0 {
            Some(x / 2)
        } else {
            Some(3 * x + 1)
        }
    })
}

fn join_display(items: impl IntoIterator<Item = impl Display>, sep: &str) -> String {
    let mut out = String::new();
    for (i, x) in items.into_iter().enumerate() {
        if i > 0 {
            out.push_str(sep);
        }
        out += &x.to_string();
    }
    out
}

fn make_filter(min: usize) -> impl Fn(&&str) -> bool {
    move |s| s.len() >= min
}

trait Source {
    fn values(&self) -> impl Iterator<Item = i32> + '_;
}
struct Data(Vec<i32>);
impl Source for Data {
    fn values(&self) -> impl Iterator<Item = i32> + '_ {
        self.0.iter().copied().filter(|x| *x > 0)
    }
}
struct Gen(i32);
impl Source for Gen {
    fn values(&self) -> impl Iterator<Item = i32> + '_ {
        (1..=self.0).map(|x| x * 100)
    }
}
fn total(s: &impl Source) -> i32 {
    s.values().sum()
}

fn main() {
    let e = evens_squared(10);
    let e2 = e.clone();
    println!("{:?} sum={}", e.collect::<Vec<_>>(), e2.sum::<u32>());
    println!("{:?}", words("Hello, world! It's 2024--ok?").collect::<Vec<_>>());
    for s in numbered(vec!['x', 'y']) {
        print!("{} ", s);
    }
    println!();
    println!("{:?}", numbered(words("a b c")).collect::<Vec<_>>());
    let mut c = counter(5);
    println!("{} {} {}", c(), c(), c());
    println!("{:?} {:?}", either_iter(true).collect::<Vec<_>>(), either_iter(false).collect::<Vec<_>>());
    println!("{:?}", pairs([1, 2, 3, 4].iter()).map(|(a, b)| a * b).collect::<Vec<_>>());
    println!("{:?}", pairs("abc".chars()).map(|(a, b)| format!("{}{}", a, b)).collect::<Vec<_>>());
    println!("{:?}", fib().take_while(|&x| x < 100).collect::<Vec<_>>());
    println!("{}", fib().nth(80).unwrap());
    println!("{:?}", collatz(27).count());
    println!("{}", collatz(6).map(|x| x.to_string()).collect::<Vec<_>>().join("->"));
    println!("{}", join_display([1.5, 2.25], " | "));
    println!("{}", join_display(vec!["a", "b", "c"], ""));
    let ws = ["hi", "hello", "hey", "greetings"];
    let f = make_filter(3);
    println!("{:?}", ws.iter().copied().filter(f).collect::<Vec<_>>());
    println!("{} {}", total(&Data(vec![3, -1, 4, -1, 5])), total(&Gen(4)));
}
