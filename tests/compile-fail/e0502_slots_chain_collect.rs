//~ ERROR E0502
fn main() {
    let a = vec![1];
    let mut b = vec![2];
    let all: Vec<&i32> = a.iter().chain(b.iter()).collect();
    b.push(3);
    println!("{:?}", all);
}
