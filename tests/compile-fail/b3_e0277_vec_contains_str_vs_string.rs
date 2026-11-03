//~ ERROR E0277
fn main() {
    let v = vec![String::from("a")];
    let found = v.iter().any(|s| s == 'a');
    println!("{}", found);
}
