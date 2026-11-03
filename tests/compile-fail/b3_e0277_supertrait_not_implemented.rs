//~ ERROR E0277
trait Named { fn name(&self) -> String; }
trait Greeter: Named { fn greet(&self) -> String { format!("hi {}", self.name()) } }
struct Bot;
impl Greeter for Bot {}
fn main() { Bot.greet(); }
