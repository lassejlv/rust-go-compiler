#[derive(Debug, Clone, Copy, PartialEq)]
enum Maybe<T> { Just(T), Nothing }
use Maybe::*;

impl<T> Maybe<T> {
    fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Maybe<U> { match self { Just(x) => Just(f(x)), Nothing => Nothing } }
    fn and_then<U>(self, f: impl FnOnce(T) -> Maybe<U>) -> Maybe<U> { match self { Just(x) => f(x), Nothing => Nothing } }
    fn unwrap_or(self, d: T) -> T { match self { Just(x) => x, Nothing => d } }
    fn unwrap_or_else(self, f: impl FnOnce() -> T) -> T { match self { Just(x) => x, Nothing => f() } }
    fn as_ref(&self) -> Maybe<&T> { match self { Just(x) => Just(x), Nothing => Nothing } }
    fn filter(self, p: impl FnOnce(&T) -> bool) -> Self { match self { Just(x) if p(&x) => Just(x), _ => Nothing } }
    fn zip<U>(self, o: Maybe<U>) -> Maybe<(T, U)> { match (self, o) { (Just(a), Just(b)) => Just((a, b)), _ => Nothing } }
    fn ok_or<E>(self, e: E) -> Outcome<T, E> { match self { Just(x) => Good(x), Nothing => Bad(e) } }
    fn take(&mut self) -> Maybe<T> { std::mem::replace(self, Nothing) }
    fn is_just(&self) -> bool { matches!(self, Just(_)) }
}
impl<T: Default> Maybe<T> { fn unwrap_or_default(self) -> T { self.unwrap_or_else(T::default) } }
impl<T> Maybe<Maybe<T>> { fn flatten(self) -> Maybe<T> { self.and_then(|x| x) } }
impl<T, E> Maybe<Outcome<T, E>> {
    fn transpose(self) -> Outcome<Maybe<T>, E> {
        match self { Nothing => Good(Nothing), Just(Good(x)) => Good(Just(x)), Just(Bad(e)) => Bad(e) }
    }
}
impl<T> From<Option<T>> for Maybe<T> { fn from(o: Option<T>) -> Self { match o { Some(x) => Just(x), None => Nothing } } }

#[derive(Debug, Clone, PartialEq)]
enum Outcome<T, E> { Good(T), Bad(E) }
use Outcome::*;
impl<T, E> Outcome<T, E> {
    fn map_err<F2>(self, f: impl FnOnce(E) -> F2) -> Outcome<T, F2> { match self { Good(x) => Good(x), Bad(e) => Bad(f(e)) } }
    fn and_then<U>(self, f: impl FnOnce(T) -> Outcome<U, E>) -> Outcome<U, E> { match self { Good(x) => f(x), Bad(e) => Bad(e) } }
    fn ok(self) -> Maybe<T> { match self { Good(x) => Just(x), Bad(_) => Nothing } }
}
impl<T, E: std::fmt::Debug> Outcome<T, E> {
    fn expect(self, msg: &str) -> T { match self { Good(x) => x, Bad(e) => panic!("{}: {:?}", msg, e) } }
}

fn collect_all<T, E>(v: Vec<Outcome<T, E>>) -> Outcome<Vec<T>, E> {
    let mut out = vec![];
    for o in v { match o { Good(x) => out.push(x), Bad(e) => return Bad(e) } }
    Good(out)
}

fn parse(s: &str) -> Outcome<i32, String> {
    match s.parse::<i32>() { Ok(n) => Good(n), Err(_) => Bad(format!("bad:{}", s)) }
}

fn main() {
    let a = Just(4);
    println!("{:?} {:?}", a.map(|x| x * 2), Nothing::<i32>.map(|x| x * 2));
    println!("{:?}", a.and_then(|x| if x > 3 { Just(x.to_string()) } else { Nothing }));
    println!("{} {}", Nothing.unwrap_or(9), a.unwrap_or_else(|| 0));
    println!("{:?} {:?}", a.filter(|x| x % 2 == 1), a.zip(Just('c')));
    println!("{:?} {:?}", a.as_ref().map(|r| *r + 1), Just(Just(1)).flatten());
    println!("{:?} {:?}", Nothing::<u8>.ok_or("none"), Maybe::<String>::Nothing.unwrap_or_default());
    let mut m = Just(String::from("x"));
    let t = m.take();
    println!("{:?} {:?} {}", t, m, t.is_just());
    println!("{:?}", Just(parse("5")).transpose());
    println!("{:?}", Just(parse("q")).transpose());
    println!("{:?}", Maybe::<Outcome<i32, String>>::Nothing.transpose());
    println!("{:?}", collect_all(vec![parse("1"), parse("2")]));
    println!("{:?}", collect_all(vec![parse("1"), parse("z"), parse("y")]));
    println!("{:?}", parse("7").and_then(|x| if x > 5 { Bad("big".to_string()) } else { Good(x) }).map_err(|e| e.len()));
    println!("{:?} {:?}", parse("3").ok(), parse("w").ok());
    let from: Maybe<char> = Some('z').into();
    println!("{:?} {:?}", from, Maybe::from(None::<i32>));
    println!("{}", parse("8").expect("should parse"));
    parse("nope").expect("failed");
}
