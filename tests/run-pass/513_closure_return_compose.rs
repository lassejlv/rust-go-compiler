// returning closures, composing, currying, memoization with closures
use std::collections::HashMap;

fn adder(n: i32) -> impl Fn(i32) -> i32 {
    move |x| x + n
}

fn multiplier(n: i32) -> Box<dyn Fn(i32) -> i32> {
    Box::new(move |x| x * n)
}

fn compose<A, B, C>(f: impl Fn(A) -> B, g: impl Fn(B) -> C) -> impl Fn(A) -> C {
    move |a| g(f(a))
}

fn curry(f: fn(i32, i32, i32) -> i32) -> impl Fn(i32) -> Box<dyn Fn(i32) -> Box<dyn Fn(i32) -> i32>> {
    move |a| Box::new(move |b| Box::new(move |c| f(a, b, c)))
}

fn power_fn(n: u32) -> Box<dyn Fn(i64) -> i64> {
    if n == 0 {
        Box::new(|_| 1)
    } else {
        let prev = power_fn(n - 1);
        Box::new(move |x| x * prev(x))
    }
}

fn memoize<F>(f: F) -> impl FnMut(u64) -> u64
where
    F: Fn(u64) -> u64,
{
    let mut cache: HashMap<u64, u64> = HashMap::new();
    move |n| *cache.entry(n).or_insert_with(|| f(n))
}

fn make_accumulators() -> (impl FnMut(i32) -> i32, impl FnMut(&str) -> String) {
    let mut sum = 0;
    let mut text = String::new();
    (
        move |x| {
            sum += x;
            sum
        },
        move |s| {
            if !text.is_empty() {
                text.push('-');
            }
            text.push_str(s);
            text.clone()
        },
    )
}

fn apply_n<F: Fn(i32) -> i32>(f: F, n: usize) -> impl Fn(i32) -> i32 {
    move |mut x| {
        for _ in 0..n {
            x = f(x);
        }
        x
    }
}

fn pipeline(steps: Vec<(&'static str, Box<dyn Fn(f64) -> f64>)>) -> impl Fn(f64) -> (f64, Vec<String>) {
    move |mut v| {
        let mut trace = vec![];
        for (name, s) in &steps {
            v = s(v);
            trace.push(format!("{}={}", name, v));
        }
        (v, trace)
    }
}

fn main() {
    let add5 = adder(5);
    let times3 = multiplier(3);
    println!("{} {}", add5(1), times3(7));
    let both = compose(&add5, &*times3);
    println!("{}", both(2));
    let to_s = compose(both, |x: i32| format!("[{}]", x));
    println!("{}", to_s(10));
    let c = curry(|a, b, c| a * 100 + b * 10 + c);
    println!("{}", c(1)(2)(3));
    let c12 = c(1)(2);
    println!("{} {}", c12(8), c12(9));
    let cube = power_fn(3);
    println!("{} {}", cube(3), power_fn(10)(2));
    let mut slow_calls = 0;
    {
        let mut sq = memoize(|n| n * n + 1);
        for n in [3, 4, 3, 3, 5, 4] {
            slow_calls += 1;
            print!("{} ", sq(n));
        }
        println!();
    }
    println!("calls {}", slow_calls);
    let (mut acc, mut cat) = make_accumulators();
    println!("{} {} {}", acc(1), acc(10), acc(-4));
    println!("{}", cat("a"));
    println!("{}", cat("bc"));
    let inc10 = apply_n(|x| x + 1, 10);
    let dbl4 = apply_n(|x| x * 2, 4);
    println!("{} {}", inc10(0), dbl4(3));
    let p = pipeline(vec![
        ("half", Box::new(|x| x / 2.0)),
        ("sq", Box::new(|x| x * x)),
        ("neg", Box::new(|x: f64| -x)),
    ]);
    let (v, t) = p(5.0);
    println!("{} {:?}", v, t);
    let fs: Vec<Box<dyn Fn(i32) -> i32>> = (1..=4).map(|k| Box::new(move |x| x * k + k) as Box<dyn Fn(i32) -> i32>).collect();
    println!("{:?}", fs.iter().map(|f| f(10)).collect::<Vec<_>>());
    let composed = fs.into_iter().fold(Box::new(|x| x) as Box<dyn Fn(i32) -> i32>, |acc, f| Box::new(move |x| f(acc(x))));
    println!("{}", composed(1));
}
