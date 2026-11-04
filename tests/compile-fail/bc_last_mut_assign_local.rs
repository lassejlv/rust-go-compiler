//~ ERROR E0597
fn main() {
    let mut v: Vec<&str> = vec!["a", "b"];
    {
        let s = String::from("x");
        if let Some(x) = v.last_mut() { *x = s.as_str(); }
        let (a, _b) = v.split_at_mut(1);
        a[0] = &s;
    }
    println!("{:?}", v);
}
