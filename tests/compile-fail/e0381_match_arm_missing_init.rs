//~ ERROR E0381
fn main() {
    let n = std::env::args().count();
    let label: &str;
    match n {
        0 => label = "zero",
        1 => label = "one",
        _ => {}
    }
    println!("{}", label);
}
