//~ ERROR E0597
fn main() {
    let mut v: Vec<&str> = vec!["a", "b"];
    let mut w: &str = "w";
    {
        let s = String::from("x");
        let x = &mut v[0];
        *x = &s;
        let y = &mut w;
        *y = &s;
    }
    println!("{:?} {}", v, w);
}
