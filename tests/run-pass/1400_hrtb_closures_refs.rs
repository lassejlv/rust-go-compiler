use std::collections::HashMap;
use std::fmt::Debug;

fn apply_to_all<F>(items: &[String], f: F) -> Vec<&str>
where
    F: for<'a> Fn(&'a String) -> &'a str,
{
    items.iter().map(|s| f(s)).collect()
}

fn first_word(s: &String) -> &str {
    s.split_whitespace().next().unwrap_or("")
}

fn sum_all<C>(c: &C) -> i64
where
    for<'a> &'a C: IntoIterator<Item = &'a i64>,
{
    let mut t = 0;
    for x in c {
        t += *x;
    }
    t
}

fn debug_all<C>(c: &C) -> String
where
    for<'a> &'a C: IntoIterator,
    for<'a> <&'a C as IntoIterator>::Item: Debug,
{
    let mut out = Vec::new();
    for x in c {
        out.push(format!("{:?}", x));
    }
    out.join("|")
}

struct Processor<'p> {
    handlers: Vec<Box<dyn for<'a> Fn(&'a str) -> Option<&'a str> + 'p>>,
}

impl<'p> Processor<'p> {
    fn new() -> Self {
        Processor { handlers: Vec::new() }
    }
    fn add(&mut self, h: impl for<'a> Fn(&'a str) -> Option<&'a str> + 'p) {
        self.handlers.push(Box::new(h));
    }
    fn run<'s>(&self, s: &'s str) -> Vec<&'s str> {
        self.handlers.iter().filter_map(|h| h(s)).collect()
    }
}

fn compose<A, B, C>(f: impl Fn(A) -> B, g: impl Fn(B) -> C) -> impl Fn(A) -> C {
    move |x| g(f(x))
}

fn call_with_ref<F, R>(f: F) -> R
where
    F: for<'x> FnOnce(&'x mut Vec<i32>) -> R,
{
    let mut v = vec![1, 2, 3];
    f(&mut v)
}

trait Visitor {
    fn visit<'a>(&mut self, s: &'a str) -> &'a str;
}

struct Trim;
impl Visitor for Trim {
    fn visit<'a>(&mut self, s: &'a str) -> &'a str {
        s.trim()
    }
}

fn run_visitor(v: &mut dyn Visitor, inputs: &[&str]) -> Vec<String> {
    inputs.iter().map(|s| v.visit(s).to_string()).collect()
}

fn longest_by<'a, T, K: Ord>(xs: &'a [T], key: impl Fn(&T) -> K) -> Option<&'a T> {
    xs.iter().max_by_key(|x| key(x))
}

fn main() {
    let items = vec!["hello world".to_string(), "foo bar baz".to_string(), "".to_string()];
    let firsts = apply_to_all(&items, first_word);
    println!("{:?}", firsts);
    let lens = apply_to_all(&items, |s: &String| s.as_str());
    println!("{:?}", lens);

    let v: Vec<i64> = vec![10, 20, 30];
    println!("sum={}", sum_all(&v));
    let arr: [i64; 4] = [1, 2, 3, 4];
    println!("sum={}", sum_all(&arr));

    let mut hm = HashMap::new();
    hm.insert(1, "a");
    println!("{}", debug_all::<Vec<Option<i32>>>(&vec![Some(1), None]));
    println!("{}", debug_all::<HashMap<i32, &str>>(&hm));

    let prefix = String::from("x");
    let mut p = Processor::new();
    p.add(|s| s.strip_prefix("x"));
    p.add(|s| if s.len() > 3 { Some(&s[..3]) } else { None });
    let pr = &prefix;
    p.add(move |s| if s.starts_with(pr.as_str()) { Some(s) } else { None });
    println!("{:?}", p.run("xylophone"));
    println!("{:?}", p.run("ab"));

    let h = compose(|x: i32| x * 2, |y: i32| format!("<{}>", y));
    println!("{}", h(21));
    let h2 = compose(|s: &str| s.len(), |n: usize| n * n);
    println!("{}", h2("abcd"));

    let r = call_with_ref(|v| {
        v.push(4);
        v.iter().sum::<i32>()
    });
    println!("{}", r);
    let l = call_with_ref(|v| v.len());
    println!("{}", l);

    let mut t = Trim;
    println!("{:?}", run_visitor(&mut t, &["  a ", "b  ", " c"]));

    let words = ["apple", "fig", "banana", "kiwi"];
    println!("{:?}", longest_by(&words, |w| w.len()));
    let nums = [3, -7, 5];
    println!("{:?}", longest_by(&nums, |n: &i32| n.abs()));
}
