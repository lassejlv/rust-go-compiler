//~ ERROR E0597
fn main() {
    let mut v: Vec<&str> = vec![];
    {
        let s = String::from("x");
        let mut f = || v.push(&s);
        f();
    }
    println!("{:?}", v);
}
