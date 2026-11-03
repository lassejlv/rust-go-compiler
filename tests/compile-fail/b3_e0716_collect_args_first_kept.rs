//~ ERROR E0716
fn main() {
    let first = std::env::args().collect::<Vec<_>>().first();
    println!("{:?}", first);
}
