//~ ERROR E0502
fn main() {
    let mut v = vec![1, 2, 3];
    let total = || v.iter().sum::<i32>();
    v.push(4);
    println!("{}", total());
}
