//~ ERROR E0277
fn main() {
    let pairs = vec![(1, 'a'), (2, 'b')];
    let (a, b): (Vec<i32>, u8) = pairs.into_iter().unzip();
    println!("{:?} {}", a, b);
}
