fn evens<I: IntoIterator<Item = u32>>(it: I) -> impl Iterator<Item = u32> { it.into_iter().filter(|x| x % 2 == 0) }
fn squares(n: u32) -> Box<dyn Iterator<Item = u32>> { Box::new((1..=n).map(|x| x * x)) }
fn main() {
    let v: Vec<u32> = evens(squares(6)).collect();
    let s: u32 = evens(vec![1, 2, 4]).chain(squares(2)).sum();
    println!("{:?} {}", v, s);
}
