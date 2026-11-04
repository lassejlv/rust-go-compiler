//~ ERROR E0597
fn main() {
    let a = String::from("a");
    let mut v = Vec::new();
    {
        let s = String::from("x");
        v.extend([(a.as_str(), s.as_str())]);
    }
    println!("{:?}", v);
}
