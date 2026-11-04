//~ ERROR E0597
fn main() {
    let mut o = None;
    {
        let s = String::from("x");
        let r: &str = &s;
        let mut f = || o = Some(r);
        f();
    }
    println!("{:?}", o);
}
