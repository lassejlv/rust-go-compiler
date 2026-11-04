//~ ERROR E0597
fn main() {
    let mut v: Vec<&str> = vec!["a", "b"];
    {
        let s = String::from("x");
        for x in v.iter_mut() { *x = &s; }
    }
    println!("{:?}", v);
}
