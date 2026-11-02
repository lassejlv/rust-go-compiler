fn wrap<I, F: FnMut(I) -> I>(mut f: F) -> impl FnMut(I) -> I { move |x| f(x) }
fn id<T>(x: T) -> T { x }
fn main() { println!("{}", wrap(id)(5)); let s: &str = wrap(id)("a"); println!("{}", s); }
