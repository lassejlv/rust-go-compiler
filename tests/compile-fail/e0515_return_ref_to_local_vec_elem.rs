//~ ERROR E0515
fn largest(n: usize) -> &'static i32 {
    let v: Vec<i32> = (0..n as i32).collect();
    v.iter().max().unwrap()
}

fn main() {
    println!("{}", largest(4));
}
