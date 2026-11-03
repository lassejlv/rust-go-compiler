// Closure-heavy code: combinators returning impl Fn, boxed FnOnce stored in
// Option and taken, FnMut passed by &mut, closures capturing closures,
// recursion via &dyn Fn (Y-combinator style), memoization with RefCell,
// and a parser-combinator mini library built from boxed closures.
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

fn compose<A, B, C>(f: impl Fn(A) -> B, g: impl Fn(B) -> C) -> impl Fn(A) -> C { move |x| g(f(x)) }
fn twice<T>(f: impl Fn(T) -> T) -> impl Fn(T) -> T { move |x| f(f(x)) }
fn curry(a: i32) -> impl Fn(i32) -> Box<dyn Fn(i32) -> i32> { move |b| Box::new(move |c| a * 100 + b * 10 + c) }

fn apply_n(f: &mut dyn FnMut() -> u32, n: usize) -> Vec<u32> { (0..n).map(|_| f()).collect() }
fn call_twice<F: FnMut(&str)>(mut f: F) { f("one"); f("two"); }

fn fix<'a>(f: &'a dyn Fn(&dyn Fn(u64) -> u64, u64) -> u64) -> impl Fn(u64) -> u64 + 'a {
    fn helper(f: &dyn Fn(&dyn Fn(u64) -> u64, u64) -> u64, x: u64) -> u64 { f(&|y| helper(f, y), x) }
    move |x| helper(f, x)
}

struct Deferred { tasks: Vec<Option<Box<dyn FnOnce() -> String>>> }
impl Deferred {
    fn run_one(&mut self, i: usize) -> Option<String> { self.tasks.get_mut(i)?.take().map(|f| f()) }
}

type P<T> = Rc<dyn Fn(&str) -> Option<(T, &str)>>;
fn chr(c: char) -> P<char> { Rc::new(move |s: &str| s.strip_prefix(c).map(|r| (c, r))) }
fn digit() -> P<u32> { Rc::new(|s: &str| { let c = s.chars().next()?; c.to_digit(10).map(|d| (d, &s[1..])) }) }
fn many<T: 'static>(p: P<T>) -> P<Vec<T>> {
    Rc::new(move |mut s: &str| {
        let mut out = vec![];
        while let Some((v, r)) = p(s) { out.push(v); s = r; }
        Some((out, s))
    })
}
fn map<T: 'static, U: 'static>(p: P<T>, f: impl Fn(T) -> U + 'static) -> P<U> { Rc::new(move |s| p(s).map(|(v, r)| (f(v), r))) }
fn seq<A: 'static, B: 'static>(a: P<A>, b: P<B>) -> P<(A, B)> {
    Rc::new(move |s| { let (x, r) = a(s)?; let (y, r2) = b(r)?; Some(((x, y), r2)) })
}
fn alt<T: 'static>(a: P<T>, b: P<T>) -> P<T> { Rc::new(move |s| a(s).or_else(|| b(s))) }
fn number() -> P<u64> {
    let ds = many(digit());
    Rc::new(move |s| { let (v, r) = ds(s)?; if v.is_empty() { None } else { Some((v.iter().fold(0u64, |a, d| a * 10 + *d as u64), r)) } })
}

fn main() {
    let inc_then_sq = compose(|x: i32| x + 1, |x| x * x);
    let sq_then_str = compose(|x: i32| x * x, |x| format!("<{}>", x));
    println!("{} {} {}", inc_then_sq(4), sq_then_str(5), twice(twice(|s: String| s + "!"))("hey".into()));
    println!("curry {}", curry(1)(2)(3));
    let adders: Vec<Box<dyn Fn(i32) -> i32>> = (1..=3).map(|k| Box::new(move |x| x + k) as Box<dyn Fn(i32) -> i32>).collect();
    println!("adders {:?}", adders.iter().map(|f| f(10)).collect::<Vec<_>>());
    let pipeline = adders.iter().fold(Box::new(|x: i32| x) as Box<dyn Fn(i32) -> i32>, |acc, f| Box::new(move |x| f(acc(x))));
    println!("pipeline {}", pipeline(0));

    let mut counter = 0;
    let mut tick = || { counter += 3; counter };
    println!("apply_n {:?}", apply_n(&mut tick, 4));
    println!("counter after {}", counter);
    let mut seen = String::new();
    call_twice(|s| { seen.push_str(s); seen.push(';'); });
    call_twice(&mut |s: &str| seen.push_str(&s.to_uppercase()));
    println!("seen {}", seen);

    let fact = |rec: &dyn Fn(u64) -> u64, n: u64| if n == 0 { 1 } else { n * rec(n - 1) };
    let f = fix(&fact);
    println!("fix fact(10)={} fact(20)={}", f(10), f(20));

    let memo: RefCell<HashMap<u64, u64>> = RefCell::new(HashMap::new());
    let calls = RefCell::new(0);
    fn memo_fib(n: u64, memo: &RefCell<HashMap<u64, u64>>, calls: &RefCell<u32>) -> u64 {
        *calls.borrow_mut() += 1;
        if n < 2 { return n; }
        if let Some(&v) = memo.borrow().get(&n) { return v; }
        let v = memo_fib(n - 1, memo, calls) + memo_fib(n - 2, memo, calls);
        memo.borrow_mut().insert(n, v);
        v
    }
    println!("memo fib(80)={} calls={}", memo_fib(80, &memo, &calls), calls.borrow());

    let owned = String::from("moved-in");
    let v = vec![1, 2, 3];
    let mut d = Deferred { tasks: vec![
        Some(Box::new(move || format!("{} {}", owned, owned.len()))),
        Some(Box::new(move || format!("sum {}", v.into_iter().sum::<i32>()))),
        None,
    ] };
    println!("{:?} {:?} {:?} {:?}", d.run_one(1), d.run_one(0), d.run_one(0), d.run_one(2));

    let sign = alt(map(chr('-'), |_| -1i64), Rc::new(|s| Some((1i64, s))));
    let int = map(seq(sign, number()), |(sg, n)| sg * n as i64);
    let ws = many(chr(' '));
    let item = map(seq(int.clone(), ws.clone()), |(n, _)| n);
    let list = many(item);
    for input in ["12 -7 300  4x", "-0 5", "abc", "99999999999 1"] {
        match list(input) {
            Some((v, rest)) => println!("{:?} -> {:?} rest {:?}", input, v, rest),
            None => println!("{:?} -> fail", input),
        }
    }
    let pair = seq(seq(chr('('), int), chr(')'));
    println!("{:?} {:?}", pair("(-42)!").map(|(((_, n), _), r)| (n, r)), pair("(4").is_some());
}
