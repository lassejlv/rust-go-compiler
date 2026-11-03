//~ ERROR E0382
fn main() {
    let v = vec![1, 2, 3];
    let doubled: Vec<i32> = v.into_iter().map(|x| x * 2).collect();
    let total: i32 = v.iter().sum();
    println!("{:?} {}", doubled, total);
}
