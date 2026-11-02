use std::fmt::{Debug, Display};

trait Entity {
    const ID: u32;
    const NAME: &'static str;
    fn tag() -> String {
        format!("{}#{}", Self::NAME, Self::ID)
    }
}

struct User;
struct Order;

impl Entity for User {
    const ID: u32 = 1;
    const NAME: &'static str = "user";
}
impl Entity for Order {
    const ID: u32 = 7;
    const NAME: &'static str = "order";
    fn tag() -> String {
        format!("ORDER-{}", Self::ID * 100)
    }
}

fn id_of<T: Entity>() -> u32 {
    T::ID
}

fn sum_ids<A: Entity, B: Entity>() -> u32 {
    A::ID + B::ID
}

struct Counter;
impl Counter {
    fn evens(limit: u32) -> impl Iterator<Item = u32> {
        (0..limit).filter(|x| x % 2 == 0)
    }
    fn labels(n: usize) -> impl Iterator<Item = impl Display> {
        (1..=n).map(|i| format!("L{}", i))
    }
}

fn numbers_as_display(v: Vec<i64>) -> impl Iterator<Item = impl Display + Clone> {
    v.into_iter().map(|x| x * 10)
}

fn print_all<I>(it: I) -> String
where
    I: IntoIterator,
    I::Item: Debug,
{
    let parts: Vec<String> = it.into_iter().map(|x| format!("{:?}", x)).collect();
    parts.join("|")
}

fn largest<I>(it: I) -> Option<I::Item>
where
    I: Iterator,
    I::Item: PartialOrd + Copy,
{
    let mut best = None;
    for x in it {
        best = match best {
            None => Some(x),
            Some(b) if x > b => Some(x),
            other => other,
        };
    }
    best
}

fn make_adder(n: i32) -> impl Fn(i32) -> i32 {
    move |x| x + n
}

fn compose<A, B, C>(f: impl Fn(A) -> B, g: impl Fn(B) -> C) -> impl Fn(A) -> C {
    move |x| g(f(x))
}

fn pairs() -> impl Iterator<Item = (usize, impl Iterator<Item = usize>)> {
    (1..4).map(|i| (i, (0..i).map(move |j| j * i)))
}

fn main() {
    println!("{} {}", User::tag(), Order::tag());
    println!("{} {}", id_of::<User>(), id_of::<Order>());
    println!("{}", sum_ids::<User, Order>());
    println!("{}", print_all(Counter::evens(9)));
    for l in Counter::labels(3) {
        print!("{} ", l);
    }
    println!();
    let disp: Vec<String> = numbers_as_display(vec![1, -2, 3]).map(|d| format!("<{}>", d)).collect();
    println!("{}", disp.concat());
    println!("{}", print_all(vec!["a", "b"]));
    println!("{}", print_all([Some(1), None]));
    println!("{:?}", largest(vec![3.5, 9.25, -1.0].into_iter()));
    println!("{:?}", largest("hello".chars()));
    println!("{:?}", largest(std::iter::empty::<i32>()));
    let add5 = make_adder(5);
    let f = compose(add5, |x| x * 2);
    let g = compose(f, |x: i32| format!("={}", x));
    println!("{}", g(10));
    for (i, inner) in pairs() {
        let v: Vec<usize> = inner.collect();
        println!("{} -> {:?}", i, v);
    }
}
