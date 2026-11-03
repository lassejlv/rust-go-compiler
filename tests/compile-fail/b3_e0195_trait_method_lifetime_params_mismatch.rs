//~ ERROR E0195
trait Merge { fn merge<'a, 'b: 'a>(&self, x: &'a str, y: &'b str) -> &'a str; }
struct P;
impl Merge for P { fn merge<'a, 'b>(&self, x: &'a str, _y: &'b str) -> &'a str { x } }
fn main() {}
