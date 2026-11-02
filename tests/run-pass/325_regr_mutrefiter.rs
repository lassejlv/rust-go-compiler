fn sum_it<I: Iterator<Item = u32>>(it: &mut I) -> u32 { it.take(2).sum() }
fn main() {
    let mut it = 1..10u32;
    println!("{} {:?}", sum_it(&mut it), it.next());
    let mut v = vec![1, 2, 3, 4].into_iter();
    let r = &mut v; let first: Vec<_> = r.take(2).collect();
    println!("{:?} {:?}", first, v.next());
}
