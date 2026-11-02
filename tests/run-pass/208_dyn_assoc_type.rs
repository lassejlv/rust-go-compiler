use std::collections::BTreeMap;
use std::fmt::Debug;

trait Source {
    type Item;
    fn pull(&mut self) -> Option<Self::Item>;
    fn name(&self) -> String;
}

struct Range {
    cur: i64,
    end: i64,
}
impl Source for Range {
    type Item = i64;
    fn pull(&mut self) -> Option<i64> {
        if self.cur < self.end {
            self.cur += 1;
            Some(self.cur - 1)
        } else {
            None
        }
    }
    fn name(&self) -> String {
        format!("range(..{})", self.end)
    }
}

struct Fib {
    a: i64,
    b: i64,
    left: u32,
}
impl Source for Fib {
    type Item = i64;
    fn pull(&mut self) -> Option<i64> {
        if self.left == 0 {
            return None;
        }
        self.left -= 1;
        let r = self.a;
        self.a = self.b;
        self.b += r;
        Some(r)
    }
    fn name(&self) -> String {
        "fib".into()
    }
}

fn drain(src: &mut dyn Source<Item = i64>) -> Vec<i64> {
    let mut v = vec![];
    while let Some(x) = src.pull() {
        v.push(x);
    }
    v
}

// Iterator trait objects with specified Item
fn boxed_iters() -> Vec<(String, Box<dyn Iterator<Item = String>>)> {
    vec![
        ("nums".into(), Box::new((1..4).map(|i| i.to_string()))),
        ("chars".into(), Box::new("xyz".chars().map(String::from))),
        ("empty".into(), Box::new(std::iter::empty())),
    ]
}

// A transform trait with associated in/out types
trait Transform {
    type In;
    type Out: Debug;
    fn apply(&self, x: Self::In) -> Self::Out;
}

struct Len;
impl Transform for Len {
    type In = String;
    type Out = usize;
    fn apply(&self, x: String) -> usize {
        x.len()
    }
}

struct Shout;
impl Transform for Shout {
    type In = String;
    type Out = String;
    fn apply(&self, x: String) -> String {
        x.to_uppercase() + "!"
    }
}

struct Parse;
impl Transform for Parse {
    type In = String;
    type Out = Result<i32, String>;
    fn apply(&self, x: String) -> Result<i32, String> {
        x.parse::<i32>().map_err(|e| e.to_string())
    }
}

fn run_all<O: Debug>(ts: &[Box<dyn Transform<In = String, Out = O>>], input: &str) -> Vec<O> {
    ts.iter().map(|t| t.apply(input.to_string())).collect()
}

// Storing heterogeneous trait objects with different assoc types behind a common trait
trait Erased {
    fn run(&self, s: &str) -> String;
}
impl<T: Transform<In = String>> Erased for T {
    fn run(&self, s: &str) -> String {
        format!("{:?}", self.apply(s.to_string()))
    }
}

fn main() {
    let mut sources: Vec<Box<dyn Source<Item = i64>>> = vec![
        Box::new(Range { cur: 3, end: 7 }),
        Box::new(Fib { a: 0, b: 1, left: 10 }),
    ];
    for s in sources.iter_mut() {
        let name = s.name();
        println!("{} -> {:?}", name, drain(s.as_mut()));
    }

    for (name, it) in boxed_iters() {
        let v: Vec<String> = it.collect();
        println!("{}: {:?}", name, v);
    }

    let lens: Vec<Box<dyn Transform<In = String, Out = usize>>> = vec![Box::new(Len), Box::new(Len)];
    println!("{:?}", run_all(&lens, "hello"));
    let strs: Vec<Box<dyn Transform<In = String, Out = String>>> = vec![Box::new(Shout)];
    println!("{:?}", run_all(&strs, "hey"));

    let erased: BTreeMap<&str, Box<dyn Erased>> = [
        ("len", Box::new(Len) as Box<dyn Erased>),
        ("shout", Box::new(Shout)),
        ("parse", Box::new(Parse)),
    ]
    .into_iter()
    .collect();
    for input in ["42", "x9"] {
        for (k, e) in &erased {
            println!("{}({}) = {}", k, input, e.run(input));
        }
    }

    let f: Box<dyn Fn(i32) -> Box<dyn Iterator<Item = i32>>> = Box::new(|n| Box::new((0..n).map(|x| x * 10)));
    println!("{:?}", f(4).collect::<Vec<_>>());
}
