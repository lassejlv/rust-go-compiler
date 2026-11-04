//~ ERROR E0597
fn main() {
    let mut v = Vec::new();
    let a = String::from("a");
    {
        let s = String::from("x");
        v.push((a.as_str(), s.as_str()));
    }
    println!("{:?}", v);
}
