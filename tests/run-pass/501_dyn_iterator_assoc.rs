// dyn Iterator<Item = ..> in various positions
use std::collections::HashMap;

fn evens(limit: u32) -> Box<dyn Iterator<Item = u32>> {
    Box::new((0..limit).filter(|x| x % 2 == 0))
}

fn pick(kind: &str) -> Box<dyn Iterator<Item = String>> {
    match kind {
        "letters" => Box::new(('a'..='e').map(|c| c.to_string())),
        "nums" => Box::new((1..4).map(|n| format!("#{}", n))),
        "empty" => Box::new(std::iter::empty()),
        _ => Box::new(std::iter::once(kind.to_uppercase())),
    }
}

struct Pipeline {
    stages: Vec<Box<dyn Fn(Box<dyn Iterator<Item = i64>>) -> Box<dyn Iterator<Item = i64>>>>,
}

impl Pipeline {
    fn new() -> Self {
        Pipeline { stages: Vec::new() }
    }
    fn add<F>(mut self, f: F) -> Self
    where
        F: Fn(Box<dyn Iterator<Item = i64>>) -> Box<dyn Iterator<Item = i64>> + 'static,
    {
        self.stages.push(Box::new(f));
        self
    }
    fn run(&self, input: Vec<i64>) -> Vec<i64> {
        let mut it: Box<dyn Iterator<Item = i64>> = Box::new(input.into_iter());
        for s in &self.stages {
            it = s(it);
        }
        it.collect()
    }
}

fn sum_dyn(it: &mut dyn Iterator<Item = i32>) -> i32 {
    let mut s = 0;
    while let Some(x) = it.next() {
        s += x;
    }
    s
}

fn count_words<'a>(it: &mut dyn Iterator<Item = &'a str>) -> Vec<(String, usize)> {
    let mut m: HashMap<String, usize> = HashMap::new();
    for w in it {
        *m.entry(w.to_lowercase()).or_default() += 1;
    }
    let mut v: Vec<_> = m.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    v
}

struct Chained {
    parts: Vec<Box<dyn Iterator<Item = char>>>,
}
impl Iterator for Chained {
    type Item = char;
    fn next(&mut self) -> Option<char> {
        while let Some(first) = self.parts.first_mut() {
            if let Some(c) = first.next() {
                return Some(c);
            }
            self.parts.remove(0);
        }
        None
    }
}

fn main() {
    let v: Vec<u32> = evens(11).collect();
    println!("{:?}", v);
    println!("{}", evens(100).skip(5).take(3).map(|x| x.to_string()).collect::<Vec<_>>().join(","));
    for k in ["letters", "nums", "empty", "other"] {
        let items: Vec<String> = pick(k).collect();
        println!("{} -> {:?}", k, items);
    }
    let p = Pipeline::new()
        .add(|it| Box::new(it.map(|x| x * 3)))
        .add(|it| Box::new(it.filter(|x| x % 2 != 0)))
        .add(|it| Box::new(it.rev_sorted()))
        .add(|it| Box::new(it.take(4)));
    println!("{:?}", p.run((1..=12).collect()));
    let mut r = 1..=10;
    println!("sum_dyn={}", sum_dyn(&mut r));
    let mut it2 = vec![5, 6, 7].into_iter().chain(std::iter::repeat(1).take(3));
    println!("sum_dyn2={}", sum_dyn(&mut it2));
    let text = "the cat The dog a cat THE end";
    let mut words = text.split_whitespace();
    println!("{:?}", count_words(&mut words));
    let ch = Chained {
        parts: vec![
            Box::new("abc".chars()),
            Box::new(std::iter::empty()),
            Box::new("XY".chars().rev()),
            Box::new(std::iter::repeat('z').take(2)),
        ],
    };
    println!("{}", ch.collect::<String>());
    let boxed: Box<dyn Iterator<Item = u8>> = Box::new(vec![3u8, 1, 2].into_iter());
    let (lo, hi) = boxed.size_hint();
    println!("hint {} {:?}", lo, hi);
    let mut peek = evens(7).peekable();
    while let Some(x) = peek.next() {
        if let Some(n) = peek.peek() {
            print!("{}<{} ", x, n);
        } else {
            println!("last {}", x);
        }
    }
}

trait RevSorted: Iterator<Item = i64> + Sized {
    fn rev_sorted(self) -> std::vec::IntoIter<i64> {
        let mut v: Vec<i64> = self.collect();
        v.sort_by(|a, b| b.cmp(a));
        v.into_iter()
    }
}
impl<I: Iterator<Item = i64>> RevSorted for I {}
