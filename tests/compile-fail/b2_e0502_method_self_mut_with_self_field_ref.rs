//~ ERROR E0502
struct Inv { items: Vec<u32>, total: u32 }
impl Inv {
    fn add(&mut self, x: &u32) { self.total += *x; self.items.push(*x); }
    fn redo(&mut self) {
        for x in self.items.iter() {
            self.add(x);
        }
    }
}
fn main() { let mut i = Inv { items: vec![1], total: 0 }; i.redo(); }
