//~ ERROR E0505
fn main() {
    let v = vec![String::from("a")];
    let refs: Vec<&str> = v.iter().map(String::as_str).collect();
    let moved = v;
    println!("{:?} {}", refs, moved.len());
}
