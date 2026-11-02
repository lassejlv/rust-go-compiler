#[derive(Debug, Clone, PartialEq)]
enum Maybe<T> {
    Just(T),
    Nothing,
}

impl<T> Maybe<T> {
    fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Maybe<U> {
        match self {
            Maybe::Just(v) => Maybe::Just(f(v)),
            Maybe::Nothing => Maybe::Nothing,
        }
    }
    fn unwrap_or(self, d: T) -> T {
        match self {
            Maybe::Just(v) => v,
            Maybe::Nothing => d,
        }
    }
    fn is_just(&self) -> bool {
        matches!(self, Maybe::Just(_))
    }
}

#[derive(Debug)]
enum Either<L, R> {
    Left(L),
    Right(R),
}

impl<L: std::fmt::Debug, R: std::fmt::Debug> Either<L, R> {
    fn describe(&self) -> String {
        match self {
            Either::Left(l) => format!("left {:?}", l),
            Either::Right(r) => format!("right {:?}", r),
        }
    }
}

#[derive(Debug)]
enum Tree<T> {
    Leaf(T),
    Node(Box<Tree<T>>, Box<Tree<T>>),
}

impl<T: Copy + std::ops::Add<Output = T>> Tree<T> {
    fn sum(&self) -> T {
        match self {
            Tree::Leaf(v) => *v,
            Tree::Node(l, r) => l.sum() + r.sum(),
        }
    }
}

impl<T> Tree<T> {
    fn depth(&self) -> usize {
        match self {
            Tree::Leaf(_) => 1,
            Tree::Node(l, r) => 1 + l.depth().max(r.depth()),
        }
    }
    fn leaves(&self) -> usize {
        match self {
            Tree::Leaf(_) => 1,
            Tree::Node(l, r) => l.leaves() + r.leaves(),
        }
    }
}

fn parse_either(s: &str) -> Either<i32, String> {
    match s.parse::<i32>() {
        Ok(n) => Either::Left(n),
        Err(_) => Either::Right(s.to_string()),
    }
}

fn main() {
    let a = Maybe::Just(5);
    let b: Maybe<i32> = Maybe::Nothing;
    println!("{:?} {:?}", a.clone().map(|x| x * 2), b.clone().map(|x| x * 2));
    println!("{} {}", a.clone().unwrap_or(0), b.clone().unwrap_or(-1));
    println!("{} {}", a.is_just(), b.is_just());
    println!("{:?}", Maybe::Just("s").map(|s| s.len()));
    println!("eq: {}", a == Maybe::Just(5));
    for s in ["12", "abc", "-4"] {
        println!("{}", parse_either(s).describe());
    }
    let t = Tree::Node(
        Box::new(Tree::Leaf(1)),
        Box::new(Tree::Node(Box::new(Tree::Leaf(2)), Box::new(Tree::Node(Box::new(Tree::Leaf(3)), Box::new(Tree::Leaf(4)))))),
    );
    println!("sum={} depth={} leaves={}", t.sum(), t.depth(), t.leaves());
    let tf = Tree::Node(Box::new(Tree::Leaf(0.5)), Box::new(Tree::Leaf(0.25)));
    println!("float sum={}", tf.sum());
    println!("{:?}", tf);
}
