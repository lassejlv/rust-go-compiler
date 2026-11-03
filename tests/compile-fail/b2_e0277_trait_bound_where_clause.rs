//~ ERROR E0277
trait Speak { fn speak(&self) -> String; }
struct Dog;
fn talk<T>(t: T) -> String where T: Speak { t.speak() }
fn main() { talk(Dog); }
