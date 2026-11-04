//~ ERROR E0605
// An integer cannot be cast to a reference.
fn main() {
    let x = 5;
    let s = x as &str;
    println!("{}", s);
}
