//~ ERROR E0283
fn main() {
    let s = "a".into();
    let _ = s;
    let n: i64 = 1i32.into();
    let _ = n;
    let x = 5u8.into();
    println!("{}", x);
}
