//~ ERROR E0308
fn pick(b: bool) -> impl std::fmt::Display {
    if b { 1 } else { "one" }
}
fn main() { println!("{}", pick(true)); }
