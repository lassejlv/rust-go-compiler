//~ ERROR E0369
fn main() {
    let a = String::from("a");
    let b = String::from("b");
    let c = &a + &b;
    println!("{}", c);
}
