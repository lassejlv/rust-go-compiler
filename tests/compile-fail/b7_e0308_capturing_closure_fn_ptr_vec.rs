//~ ERROR E0308
// A table of fn pointers cannot hold a capturing closure.
fn main() {
    let s = String::from("abc");
    let table: Vec<fn() -> usize> = vec![move || s.len()];
    println!("{}", table[0]());
}
