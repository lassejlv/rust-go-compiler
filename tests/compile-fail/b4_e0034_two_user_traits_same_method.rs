//~ ERROR E0034
trait Render { fn draw(&self) -> String { "render".into() } }
trait Paint { fn draw(&self) -> String { "paint".into() } }
struct Canvas;
impl Render for Canvas {}
impl Paint for Canvas {}
fn main() {
    let c = Canvas;
    println!("{}", c.draw());
}
