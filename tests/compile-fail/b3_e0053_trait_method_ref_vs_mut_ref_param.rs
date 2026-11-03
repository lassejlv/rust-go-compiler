//~ ERROR E0053
trait Visitor { fn visit(&mut self, node: &String); }
struct V;
impl Visitor for V { fn visit(&mut self, node: &mut String) { node.clear(); } }
fn main() {}
