//~ ERROR E0606
fn main() {
    let flag = true;
    let x = flag as f64;
    println!("{}", x);
}
