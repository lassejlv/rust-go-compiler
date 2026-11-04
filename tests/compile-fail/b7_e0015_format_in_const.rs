//~ ERROR E0015
// `format!` allocates a String.
const GREETING: &str = &format!("hello {}", 1);
fn main() {
    println!("{}", GREETING);
}
