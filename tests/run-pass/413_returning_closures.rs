// Returning closures: impl Fn, Box<dyn FnMut>, closures returning closures (currying),
// closure factories capturing state, and memoization via returned FnMut.
use std::collections::HashMap;

fn adder(n: i32) -> impl Fn(i32) -> i32 {
    move |x| x + n
}

fn counter(start: u32, step: u32) -> impl FnMut() -> u32 {
    let mut cur = start;
    move || {
        let v = cur;
        cur += step;
        v
    }
}

fn make_op(name: &str) -> Box<dyn Fn(f64, f64) -> f64> {
    match name {
        "add" => Box::new(|a, b| a + b),
        "sub" => Box::new(|a, b| a - b),
        "hyp" => Box::new(|a, b| (a * a + b * b).sqrt()),
        other => {
            let msg = other.to_string();
            Box::new(move |_, _| msg.len() as f64 * -1.0)
        }
    }
}

fn curry(a: i64) -> impl Fn(i64) -> Box<dyn Fn(i64) -> i64> {
    move |b| Box::new(move |c| a * 100 + b * 10 + c)
}

fn memoize(f: fn(u64) -> u64) -> impl FnMut(u64) -> (u64, bool) {
    let mut cache: HashMap<u64, u64> = HashMap::new();
    move |n| {
        if let Some(&v) = cache.get(&n) {
            (v, true)
        } else {
            let v = f(n);
            cache.insert(n, v);
            (v, false)
        }
    }
}

fn slow_square(n: u64) -> u64 {
    (0..n).map(|_| n).sum()
}

fn pipeline(stages: Vec<Box<dyn Fn(i32) -> i32>>) -> impl Fn(i32) -> i32 {
    move |x| stages.iter().fold(x, |acc, s| s(acc))
}

fn accumulator() -> Box<dyn FnMut(i32) -> Vec<i32>> {
    let mut seen = Vec::new();
    Box::new(move |x| {
        seen.push(x);
        seen.clone()
    })
}

fn main() {
    let add5 = adder(5);
    let add_neg = adder(-7);
    println!("{} {} {}", add5(1), add_neg(1), add5(add_neg(100)));

    let mut c = counter(10, 3);
    let firsts: Vec<u32> = (0..4).map(|_| c()).collect();
    println!("{:?}", firsts);
    let mut c2 = counter(0, 1);
    c2();
    println!("c2 next {} c next {}", c2(), c());

    for n in ["add", "sub", "hyp", "unknown"] {
        let f = make_op(n);
        println!("{}(3,4) = {}", n, f(3.0, 4.0));
    }

    let f = curry(1);
    let g = f(2);
    println!("curry: {} {} {}", g(3), f(9)(9), curry(4)(0)(7));

    let mut m = memoize(slow_square);
    for n in [5, 7, 5, 12, 7] {
        let (v, hit) = m(n);
        println!("sq({}) = {} cached={}", n, v, hit);
    }

    let p = pipeline(vec![Box::new(|x| x + 1), Box::new(|x| x * 10), Box::new(adder(-3))]);
    println!("pipeline: {} {}", p(0), p(4));

    let mut acc = accumulator();
    acc(1);
    acc(2);
    println!("acc: {:?}", acc(3));

    let makers: Vec<fn(i32) -> Box<dyn Fn(i32) -> i32>> = vec![
        |k| Box::new(move |x| x * k),
        |k| Box::new(move |x| x - k),
    ];
    let built: Vec<Box<dyn Fn(i32) -> i32>> = makers.iter().map(|mk| mk(4)).collect();
    println!("built: {:?}", built.iter().map(|b| b(10)).collect::<Vec<_>>());
}
