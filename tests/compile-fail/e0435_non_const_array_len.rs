//~ ERROR E0435
fn main() {
    let n = 3;
    let a = [0; n];
    println!("{}", a.len());
}
