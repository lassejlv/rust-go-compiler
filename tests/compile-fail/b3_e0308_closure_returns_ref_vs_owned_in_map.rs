//~ ERROR E0308
fn main() {
    let v = vec![String::from("a")];
    let out: Vec<String> = v.iter().map(|s| s.as_str()).collect::<Vec<&str>>();
    println!("{:?}", out);
}
