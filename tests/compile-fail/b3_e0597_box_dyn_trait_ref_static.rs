//~ ERROR E0597
trait Greet { fn hi(&self) -> String; }
struct P<'a>(&'a str);
impl<'a> Greet for P<'a> { fn hi(&self) -> String { self.0.to_string() } }
fn store(_g: Box<dyn Greet>) {}
fn main() {
    let name = String::from("n");
    store(Box::new(P(&name)));
}
