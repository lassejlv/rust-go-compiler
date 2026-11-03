//~ ERROR E0382
fn take(v: Vec<i32>) -> i32 {
    v.iter().sum()
}

fn main() {
    let v = vec![1, 2, 3];
    for _ in 0..3 {
        println!("{}", take(v));
    }
}
