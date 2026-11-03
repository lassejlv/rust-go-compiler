//~ ERROR lifetime
trait Show { fn show(&self); }
impl Show for &str { fn show(&self) { println!("{}", self); } }
fn boxed<'a>(s: &'a str) -> Box<dyn Show> {
    Box::new(s)
}
fn main() { boxed("x").show(); }
