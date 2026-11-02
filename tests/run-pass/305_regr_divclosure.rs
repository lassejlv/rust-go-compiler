fn main() {
    let x: Option<i64> = Some(3);
    let p = || panic!("empty {}", 1);
    let a = x.unwrap_or_else(p);
    let b = Some(4i64).map(|v| v + 1).unwrap_or_else(p);
    println!("{} {}", a, b);
}
