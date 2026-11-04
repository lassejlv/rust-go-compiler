//~ ERROR E0502
fn main() {
    let mut o = None;
    let mut s = String::from("x");
    {
        let mut f = || o = Some(&s);
        f();
    }
    s.push('y');
    println!("{:?}", o);
}
