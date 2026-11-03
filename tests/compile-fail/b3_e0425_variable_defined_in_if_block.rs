//~ ERROR E0425
fn main() {
    if true { let found = 1; }
    println!("{}", found);
}
