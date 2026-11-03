//~ ERROR E0106
fn make() -> &str {
    "x"
}

fn main() {
    println!("{}", make());
}
