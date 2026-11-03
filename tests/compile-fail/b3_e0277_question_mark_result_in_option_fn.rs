//~ ERROR E0277
fn first_num(s: &str) -> Option<i32> {
    let n: i32 = s.parse()?;
    Some(n)
}
fn main() { first_num("1"); }
