//~ ERROR E0277
fn main() {
    let v = vec![String::from("a")];
    let total: String = v.iter().sum();
    println!("{}", total);
}
