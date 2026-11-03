//~ ERROR E0283
fn main() {
    let v = vec![1, 2];
    let total = v.iter().sum();
    println!("{}", total);
}
