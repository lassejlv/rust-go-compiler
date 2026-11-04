//~ ERROR overflowing_literals
fn main() {
    let v: Vec<i32> = vec![1, 2, 3_000_000_000];
    println!("{:?}", v);
}
