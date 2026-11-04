use std::collections::HashMap;
use std::fmt::Debug;

fn adders(n: i32) -> impl Iterator<Item = impl Fn(i32) -> i32> {
    (0..n).map(|k| move |x| x + k)
}

fn pipeline() -> impl Fn(i32) -> Box<dyn Iterator<Item = i32>> {
    |n| Box::new((0..n).filter(|x| x % 2 == 1).map(|x| x * x))
}

fn counter() -> impl FnMut() -> u32 {
    let mut c = 0;
    move || { c += 1; c }
}

fn make_iters<'a>(v: &'a [i32]) -> (impl Iterator<Item = &'a i32> + Clone, impl Iterator<Item = i32> + 'a) {
    (v.iter().filter(|x| **x > 0), v.iter().rev().map(|x| x * 10))
}

fn curry3(a: i32) -> impl Fn(i32) -> Box<dyn Fn(i32) -> i32> {
    move |b| Box::new(move |c| a * 100 + b * 10 + c)
}

fn nested_closures() -> i64 {
    let f = |a: i64| {
        move |b: i64| {
            move |c: i64| {
                move |d: i64| {
                    move |e: i64| a + 2 * b + 3 * c + 4 * d + 5 * e
                }
            }
        }
    };
    f(1)(2)(3)(4)(5)
}

fn deep_chain(v: &[i32]) -> Vec<(usize, String)> {
    v.iter()
        .copied()
        .filter(|x| *x != 0)
        .map(|x| x * 3)
        .enumerate()
        .skip(1)
        .step_by(1)
        .take_while(|(i, _)| *i < 100)
        .map(|(i, x)| (i, x as i64))
        .filter_map(|(i, x)| if x % 2 == 0 { Some((i, x)) } else { Some((i, -x)) })
        .flat_map(|(i, x)| std::iter::repeat((i, x)).take(2))
        .scan(0i64, |acc, (i, x)| { *acc += x; Some((i, *acc)) })
        .inspect(|_| {})
        .peekable()
        .map(|(i, acc)| (i, format!("{}", acc)))
        .chain(std::iter::once((999, "end".to_string())))
        .zip(1..)
        .map(|((i, s), k)| (i * k, s))
        .collect()
}

fn group_by_len<'a, I>(words: I) -> impl Iterator<Item = (usize, Vec<&'a str>)>
where
    I: IntoIterator<Item = &'a str>,
{
    let mut m: HashMap<usize, Vec<&'a str>> = HashMap::new();
    for w in words { m.entry(w.len()).or_default().push(w); }
    let mut v: Vec<_> = m.into_iter().collect();
    v.sort();
    v.into_iter()
}

trait Shape { fn area(&self) -> f64; }
struct Sq(f64);
impl Shape for Sq { fn area(&self) -> f64 { self.0 * self.0 } }
fn shapes() -> impl Iterator<Item = impl Shape + Debug> {
    #[derive(Debug)] struct Circ(f64);
    impl Shape for Circ { fn area(&self) -> f64 { 3.0 * self.0 * self.0 } }
    (1..4).map(|r| Circ(r as f64))
}

fn compose_all<T>(fs: Vec<Box<dyn Fn(T) -> T>>) -> impl Fn(T) -> T {
    move |x| fs.iter().fold(x, |acc, f| f(acc))
}

fn returns_closure_ref<'a>(s: &'a str) -> impl Fn(usize) -> &'a str + 'a {
    move |n| &s[..n.min(s.len())]
}

fn main() {
    let fs: Vec<_> = adders(4).collect();
    println!("{:?}", fs.iter().map(|f| f(10)).collect::<Vec<_>>());
    let p = pipeline();
    println!("{:?}", p(8).collect::<Vec<_>>());
    let mut c = counter();
    c(); c();
    println!("{}", c());
    let data = [3, -1, 4, -5, 9];
    let (pos, rev) = make_iters(&data);
    let pos2 = pos.clone();
    println!("{:?} {} {:?}", pos.collect::<Vec<_>>(), pos2.count(), rev.collect::<Vec<_>>());
    println!("{}", curry3(1)(2)(3));
    println!("{}", nested_closures());
    println!("{:?}", deep_chain(&[1, 0, 2, 3, 4]));
    for (k, ws) in group_by_len("a bb cc d eee".split(' ')) {
        println!("{} -> {:?}", k, ws);
    }
    for s in shapes() { println!("{:?} {:.1}", s, s.area()); }
    println!("{}", Sq(2.0).area());
    let f = compose_all(vec![Box::new(|x: i32| x + 1), Box::new(|x| x * 10), Box::new(|x| x - 3)]);
    println!("{}", f(4));
    let g = compose_all::<String>(vec![Box::new(|s| s + "a"), Box::new(|s| s.repeat(2))]);
    println!("{}", g("z".to_string()));
    let s = String::from("hello");
    let pre = returns_closure_ref(&s);
    println!("{} {} {}", pre(2), pre(10), pre(0).is_empty());
}
