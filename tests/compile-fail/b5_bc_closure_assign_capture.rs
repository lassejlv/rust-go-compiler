//~ ERROR E0597
fn main() {
    let mut o = None;
    {
        let s = String::from("x");
        let mut f = || o = Some(&s);
        f();
    }
    println!("{:?}", o);
}
