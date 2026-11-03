use std::fmt::{Debug, Display};

#[derive(Debug, Clone, PartialEq)]
struct Pair<A, B> {
    a: A,
    b: B,
}

impl<A: Display, B: Display> Pair<A, B> {
    fn show(&self) -> String {
        format!("<{} | {}>", self.a, self.b)
    }
}

impl<A: Clone, B: Clone> Pair<A, B> {
    fn swap(&self) -> Pair<B, A> {
        Pair { a: self.b.clone(), b: self.a.clone() }
    }
}

impl<A, B> Pair<A, B> {
    fn map_a<C, F: FnOnce(A) -> C>(self, f: F) -> Pair<C, B> {
        Pair { a: f(self.a), b: self.b }
    }
    fn map_b<C>(self, f: impl FnOnce(B) -> C) -> Pair<A, C> {
        Pair { a: self.a, b: f(self.b) }
    }
}

impl<T: PartialOrd + Copy> Pair<T, T> {
    fn max(&self) -> T {
        if self.a > self.b { self.a } else { self.b }
    }
}

enum Either<L, R> {
    Left(L),
    Right(R),
}

impl<L: Debug, R: Debug> Either<L, R> {
    fn describe(&self) -> String {
        match self {
            Either::Left(l) => format!("Left({:?})", l),
            Either::Right(r) => format!("Right({:?})", r),
        }
    }
    fn flip(self) -> Either<R, L> {
        match self {
            Either::Left(l) => Either::Right(l),
            Either::Right(r) => Either::Left(r),
        }
    }
}

fn partition<T, L, R, F>(items: Vec<T>, f: F) -> (Vec<L>, Vec<R>)
where
    F: Fn(T) -> Either<L, R>,
{
    let mut ls = Vec::new();
    let mut rs = Vec::new();
    for it in items {
        match f(it) {
            Either::Left(l) => ls.push(l),
            Either::Right(r) => rs.push(r),
        }
    }
    (ls, rs)
}

fn main() {
    let p = Pair { a: 3, b: "three" };
    println!("{}", p.show());
    let q = p.swap();
    println!("{}", q.show());
    println!("{:?}", q);
    let r = q.map_a(|s| s.len()).map_b(|n| n as f64 * 1.5);
    println!("{:?}", r);
    println!("{}", Pair { a: 2.5, b: 7.25 }.max());
    println!("{}", Pair { a: 'z', b: 'a' }.max());

    let e: Either<i32, String> = Either::Left(5);
    println!("{}", e.describe());
    let f = e.flip();
    println!("{}", f.describe());
    let g: Either<i32, String> = Either::Right("hi".to_string());
    println!("{}", g.flip().describe());

    let (evens, odds): (Vec<i32>, Vec<String>) = partition((1..=10).collect(), |x| {
        if x % 2 == 0 { Either::Left(x * 10) } else { Either::Right(format!("o{}", x)) }
    });
    println!("{:?} {:?}", evens, odds);
    println!("{}", Pair { a: 1, b: 2 } == Pair { a: 1, b: 2 });
}
