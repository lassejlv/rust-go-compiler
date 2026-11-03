//~ ERROR E0119
trait Describe { fn d(&self) -> String; }
impl<T: std::fmt::Debug> Describe for T { fn d(&self) -> String { format!("{:?}", self) } }
impl Describe for i32 { fn d(&self) -> String { "int".into() } }
fn main() {}
