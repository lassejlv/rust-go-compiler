//~ ERROR E0716
struct P<'a> { s: &'a String }
impl<'a> P<'a> { fn new(s: &'a String) -> Self { P { s } } }
fn main() {
    let p = P::new(&String::from("t"));
    println!("{}", p.s);
}
