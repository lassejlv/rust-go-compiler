struct S { a: i32, #[cfg(feature = "kv")] b: Nope }
fn main() {
    let s = S { a: 1, #[cfg(feature = "kv")] b: Nope(3) };
    println!("{}", s.a);
}
