//~ ERROR E0434
fn outer(x: i32) -> i32 {
    fn inner() -> i32 { x * 2 }
    inner()
}
fn main() { outer(1); }
