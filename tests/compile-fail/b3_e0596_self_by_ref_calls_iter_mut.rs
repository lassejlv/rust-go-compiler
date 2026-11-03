//~ ERROR E0596
struct Scores { v: Vec<u32> }
impl Scores {
    fn reset(&self) { for s in self.v.iter_mut() { *s = 0; } }
}
fn main() { Scores { v: vec![] }.reset(); }
