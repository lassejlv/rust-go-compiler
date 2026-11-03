//~ ERROR E0515
fn evens() -> impl Iterator<Item = &'static i32> {
    let v = vec![2, 4, 6];
    v.iter()
}
fn main() { for x in evens() { println!("{}", x); } }
