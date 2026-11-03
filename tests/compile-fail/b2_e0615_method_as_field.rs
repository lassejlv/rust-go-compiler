//~ ERROR E0615
struct S { v: Vec<i32> }
impl S { fn len(&self) -> usize { self.v.len() } }
fn main() { let s = S { v: vec![] }; println!("{}", s.len); }
