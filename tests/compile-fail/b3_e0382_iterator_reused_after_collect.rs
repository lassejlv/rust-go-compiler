//~ ERROR E0382
fn main() {
    let it = vec![1, 2, 3].into_iter();
    let a: Vec<i32> = it.collect();
    let b: Vec<i32> = it.collect();
    println!("{:?} {:?}", a, b);
}
