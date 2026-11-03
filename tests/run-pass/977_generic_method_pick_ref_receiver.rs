// Method calls in a generic body resolve once, against the bounds, not per
// instance: with `x: &T` and no `T: Clone`, `x.clone()` is
// `<&T as Clone>::clone` (a `&T`), even when T = i32 has its own clone.

trait Hi {
    fn hi(&self) -> &'static str;
}
impl<T: ?Sized> Hi for &T {
    fn hi(&self) -> &'static str {
        "&T"
    }
}
impl Hi for i32 {
    fn hi(&self) -> &'static str {
        "i32"
    }
}

fn last<T>(v: &[T]) -> Option<&T> {
    v.last().map(|x| x.clone())
}

fn same<T: std::fmt::Debug>(x: &T) -> String {
    let y: &T = x.clone();
    format!("{:?}", y)
}

fn owned<T>(x: &T) -> &T {
    x.to_owned()
}

fn owned2<T: Clone>(x: &T) -> T {
    x.to_owned()
}

fn refs<T>(v: &Vec<T>) -> Vec<&T> {
    v.iter().map(|x| x.clone()).collect()
}

fn hi_unbounded<T>(x: &T) -> &'static str {
    x.hi()
}

fn hi_bounded<T: Hi>(x: &T) -> &'static str {
    x.hi()
}

#[derive(Debug, Clone, PartialEq)]
struct P(u8);

fn main() {
    println!("{:?}", last(&[1, 2, 3]));
    println!("{:?}", last(&[String::from("a"), String::from("b")]));
    println!("{:?}", last::<P>(&[]));
    println!("{}", same(&5u8));
    println!("{}", same(&P(4)));
    println!("{} {}", owned(&3), owned2(&4));
    println!("{:?}", refs(&vec![P(1), P(2)]));
    println!("{} {}", hi_unbounded(&1), hi_bounded(&1));
    println!("{}", (&1).hi());
}
