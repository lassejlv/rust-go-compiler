//~ ERROR E0308
fn main() {
    let c = true;
    let v = if c { 1 } else { "one" };
    println!("{}", v);
}
