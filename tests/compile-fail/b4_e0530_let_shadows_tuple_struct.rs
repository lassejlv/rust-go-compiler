//~ ERROR E0530
struct Meters(f64);
fn main() {
    let Meters = 5.0;
    println!("{}", Meters);
}
