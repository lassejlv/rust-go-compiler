//~ ERROR E0416
fn main() {
    let (a, a) = (1, 2);
    println!("{}", a);
}
