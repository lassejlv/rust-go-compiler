//~ ERROR E0381
fn main() {
    let s: String;
    let f = || s.len();
    println!("{}", f());
}
