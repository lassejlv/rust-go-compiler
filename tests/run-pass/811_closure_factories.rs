use std::collections::HashMap;

fn make_adder(n: i32) -> impl Fn(i32) -> i32 { move |x| x + n }
fn make_counter() -> impl FnMut() -> u32 { let mut c = 0; move || { c += 1; c } }
fn compose<A, B, C>(f: impl Fn(A) -> B, g: impl Fn(B) -> C) -> impl Fn(A) -> C { move |x| g(f(x)) }
fn boxed_ops() -> Vec<(&'static str, Box<dyn Fn(i64, i64) -> i64>)> {
    vec![("add", Box::new(|a, b| a + b)), ("sub", Box::new(|a, b| a - b)), ("pow", Box::new(|a, b| a.pow(b as u32))), ("max", Box::new(std::cmp::max))]
}
fn apply_n<F: FnMut()>(n: usize, mut f: F) { for _ in 0..n { f(); } }
fn call_once<F: FnOnce() -> String>(f: F) -> String { f() }
fn curry(a: i32) -> impl Fn(i32) -> Box<dyn Fn(i32) -> i32> { move |b| Box::new(move |c| a * 100 + b * 10 + c) }

struct EventBus { handlers: Vec<Box<dyn FnMut(&str) -> Option<String>>> }
impl EventBus {
    fn emit(&mut self, ev: &str) -> Vec<String> { self.handlers.iter_mut().filter_map(|h| h(ev)).collect() }
}

fn memoize(f: fn(u64) -> u64) -> impl FnMut(u64) -> u64 {
    let mut cache: HashMap<u64, u64> = HashMap::new();
    move |n| *cache.entry(n).or_insert_with(|| f(n))
}

fn slow_square(n: u64) -> u64 { n * n }

fn pipeline(stages: Vec<Box<dyn Fn(Vec<i32>) -> Vec<i32>>>) -> impl Fn(Vec<i32>) -> Vec<i32> {
    move |input| stages.iter().fold(input, |acc, s| s(acc))
}

fn main() {
    let add5 = make_adder(5);
    let times3 = |x: i32| x * 3;
    let both = compose(add5, times3);
    let tostr = compose(&both, |x: i32| format!("<{}>", x));
    println!("{} {} {}", both(1), both(-5), tostr(10));
    let mut c1 = make_counter();
    let mut c2 = make_counter();
    println!("{} {} {} {}", c1(), c1(), c2(), c1());
    for (name, op) in boxed_ops().iter() { print!("{}={} ", name, op(3, 4)); }
    println!();
    let mut total = 0;
    apply_n(4, || total += 7);
    println!("total {}", total);
    let s = String::from("owned");
    let consumed = call_once(move || s + "!");
    println!("{}", consumed);
    println!("curry {}", curry(1)(2)(3));
    let adders: Vec<Box<dyn Fn(i32) -> i32>> = (1..=3).map(|k| Box::new(move |x| x * k) as Box<dyn Fn(i32) -> i32>).collect();
    println!("{:?}", adders.iter().map(|f| f(10)).collect::<Vec<_>>());

    let mut log_count = 0usize;
    let mut bus = EventBus { handlers: vec![] };
    bus.handlers.push(Box::new(|e| if e.starts_with("err") { Some(format!("ALERT:{}", e)) } else { None }));
    let mut seen = Vec::new();
    bus.handlers.push(Box::new(move |e| { seen.push(e.len()); Some(format!("seen{:?}", seen)) }));
    bus.handlers.push(Box::new(move |e| { log_count += 1; if log_count % 2 == 0 { Some(format!("log#{} {}", log_count, e)) } else { None } }));
    for ev in ["start", "err:disk", "tick", "err:net"] {
        println!("{} -> {:?}", ev, bus.emit(ev));
    }
    let mut msq = memoize(slow_square);
    println!("memo {} {} {}", msq(12), msq(12), msq(3));
    let p = pipeline(vec![
        Box::new(|v| v.into_iter().filter(|x| x % 2 == 1).collect()),
        Box::new(|v| v.iter().map(|x| x * x).collect()),
        Box::new(|mut v| { v.reverse(); v }),
    ]);
    println!("pipeline {:?}", p((1..10).collect()));
    let mut v = vec![3, 1, 2];
    let mut sorter = |desc: bool| { if desc { v.sort_by(|a, b| b.cmp(a)) } else { v.sort() } ; v.clone() };
    println!("{:?} {:?}", sorter(true), sorter(false));
    let x = 10;
    let by_ref = || x + 1;
    let by_move = move || x + 2;
    println!("{} {} {}", by_ref(), by_move(), x);
    let mut names = vec!["b".to_string(), "a".to_string()];
    let mut push = |s: &str| names.push(s.to_uppercase());
    push("c"); push("d");
    names.sort();
    println!("{:?}", names);
    let fns: [fn(f64) -> f64; 3] = [f64::sqrt, f64::abs, |x| x * 2.0];
    println!("{:?}", fns.iter().map(|f| f(16.0)).collect::<Vec<_>>());
    let make_greeter = |greeting: String| move |name: &str| format!("{}, {}!", greeting, name);
    let hi = make_greeter("Hi".into());
    let yo = make_greeter("Yo".into());
    println!("{} {}", hi("Ann"), yo("Bo"));
    let mut state = 1u64;
    let mut lcg = move || { state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); (state >> 33) % 100 };
    let rolls: Vec<u64> = (0..8).map(|_| lcg()).collect();
    println!("lcg {:?}", rolls);
    let fib = {
        fn fib_inner(n: u32) -> u64 { if n < 2 { n as u64 } else { fib_inner(n - 1) + fib_inner(n - 2) } }
        fib_inner
    };
    println!("fib {}", fib(20));
    let opt_f: Option<Box<dyn Fn(i32) -> i32>> = Some(Box::new(|x| -x));
    println!("{:?}", opt_f.as_ref().map(|f| f(4)));
}
