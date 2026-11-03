//~ ERROR E0271
fn main() {
    let v = vec![String::from("a")];
    let c: Vec<String> = v.into_iter().cloned().collect();
    println!("{:?}", c);
}
